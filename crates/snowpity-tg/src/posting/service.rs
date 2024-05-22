use crate::posting::platform::prelude::*;
use crate::posting::tg_upload;
use crate::posting::Request;
use crate::prelude::*;
use crate::{http, posting};
use crate::{tg, util, Result};
use futures::future::BoxFuture;
use futures::prelude::*;
use futures::stream::FuturesUnordered;
use std::collections::HashMap;
use std::fmt;
use std::ops::ControlFlow;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};

/// Maximum number of in-flight cache requests, otherwise the service will
/// block the new requests.
const MAX_IN_FLIGHT: usize = 40;
const UNEXPECTED_SERVICE_SHUTDOWN: &str = "BUG: Service exited unexpectedly";

metrics_bat::gauges! {
    /// Number of in-flight requests for blobs cache
    blob_cache_requests_in_flight_total;
}

metrics_bat::counters! {
    /// Number of times we hit the database cache for blobs
    blob_cache_hits_total;

    /// Number of times we queried the database cache for blobs
    blob_cache_requests_total;
}

#[derive(Debug)]
pub(crate) struct CachePostRequest {
    pub(crate) requested_by: teloxide::types::User,
    pub(crate) request: Request,
    pub(crate) mirror: Option<posting::Mirror>,
}

pub(crate) struct Envelope {
    request: CachePostRequest,
    return_slot: oneshot::Sender<Result<CachedPost>>,
}

impl fmt::Debug for Envelope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            request,
            return_slot,
        } = self;

        f.debug_struct("Envelope")
            .field("request", request)
            .field("return_slot", &util::type_name_of_val(return_slot))
            .finish()
    }
}

pub(crate) fn spawn_service(ctx: PostingContext) -> PostingServiceHandle {
    let (send, recv) = mpsc::channel(MAX_IN_FLIGHT);
    let service = PostingService {
        ctx,
        in_flight_futs: Default::default(),
        return_slots: Default::default(),
        requests: recv,
    };
    PostingServiceHandle {
        send: Some(send),
        join_handle: Some(tokio::spawn(service.run_loop())),
    }
}

pub(crate) struct PostingServiceHandle {
    send: Option<mpsc::Sender<Envelope>>,
    join_handle: Option<tokio::task::JoinHandle<()>>,
}

#[derive(Clone)]
pub(crate) struct PostingContext {
    pub(super) bot: tg::Bot,
    pub(super) config: Arc<tg::Config>,
    pub(super) http: http::Client,
    pub(super) platforms: Arc<posting::AllPlatforms>,
}

struct PostingService {
    ctx: PostingContext,

    /// Pool of in-flight requests to the blob cache
    in_flight_futs: FuturesUnordered<BoxFuture<'static, (Request, Result<CachedPost>)>>,

    /// Multiplexes the results of the cache requests to the callers
    return_slots: HashMap<Request, Vec<ReturnSlot>>,

    /// Channel for incoming requests
    requests: mpsc::Receiver<Envelope>,
}

struct ReturnSlot {
    mirror: Option<posting::Mirror>,
    send: oneshot::Sender<Result<CachedPost>>,
}

impl PostingServiceHandle {
    /// Resolves a post with the telegram file ids for all blobs attached to the post.
    ///
    /// It maintains a cache of blobs, that were already requested, using
    /// a database, and saving the files in a dedicated telegram channel,
    /// if the blob is seen for the first time.
    ///
    /// It's totally fine to call this method concurrently and with the same
    /// `request_id` repeatedly, but there is a backpressure mechanism so that
    /// the future won't resolve until the service's capacity is available.
    pub(crate) async fn cache_post(&self, request: CachePostRequest) -> Result<CachedPost> {
        let (request, recv) = Envelope::new(request);
        self.send
            .as_ref()
            .expect("BUG: `send` is set to `None` only in `Drop`")
            .send(request)
            .await
            .expect(UNEXPECTED_SERVICE_SHUTDOWN);
        recv.await
    }
}

impl Drop for PostingServiceHandle {
    fn drop(&mut self) {
        // Drop the sender to signal the service to exit.
        self.send = None;
        util::tokio::block_in_place(self.join_handle.take().unwrap());
    }
}

impl Envelope {
    fn new(request: CachePostRequest) -> (Self, impl Future<Output = Result<CachedPost>>) {
        let (send, recv) = oneshot::channel();
        let me = Self {
            request,
            return_slot: send,
        };
        (me, recv.map(|val| val.expect(UNEXPECTED_SERVICE_SHUTDOWN)))
    }
}

impl PostingService {
    #[instrument(skip(self))]
    async fn run_loop(mut self) {
        loop {
            let result = std::panic::AssertUnwindSafe(self.loop_turn())
                .catch_unwind()
                .await;

            match result {
                Ok(ControlFlow::Break(())) => break,
                Ok(ControlFlow::Continue(())) => {}
                Err(_) => error!("BUG: media cache service panicked, but will continue to run"),
            }
        }
    }

    async fn loop_turn(&mut self) -> ControlFlow<()> {
        let total_in_flight = self.total_in_flight();
        blob_cache_requests_in_flight_total(vec![]).set(total_in_flight as f64);

        tokio::select! {
            // This `if` condition implements a simple backpressure mechanism
            // to prevent receiving new requests when the number of in-flight
            // requests is too high.
            request = self.requests.recv(), if total_in_flight <= MAX_IN_FLIGHT => {
                let Some(request) = request else {
                    info!("Exiting media cache service (channel closed)...");
                    return ControlFlow::Break(());
                };
                self.process_request(request);
            }
            Some((media_id, response)) = self.in_flight_futs.next() => {
                self.dispatch_response(media_id, response);
            }
        }

        ControlFlow::Continue(())
    }

    fn total_in_flight(&self) -> usize {
        self.return_slots
            .values()
            .map(|senders| senders.len())
            .sum::<usize>()
    }

    #[instrument(skip(self, response))]
    fn dispatch_response(&mut self, request: Request, response: Result<CachedPost>) {
        let slots = self
            .return_slots
            .remove(&request)
            .expect("BUG: an in-flight future must have a corresponding response return slot");

        for slot in slots {
            let mut response = response.clone();

            // Update the mirror from the original request with the mirror required
            // for this particular request (we saved it in the return slot for that).
            if let Ok(post) = &mut response {
                post.mirror = slot.mirror;
            }

            if slot.send.send(response).is_err() {
                warn!("Failed to send response because the receiver has been dropped");
            }
        }
    }

    #[instrument(skip(self))]
    fn process_request(&mut self, request: Envelope) {
        let Envelope {
            request,
            return_slot,
        } = request;

        let return_slot = ReturnSlot {
            mirror: request.mirror.clone(),
            send: return_slot,
        };

        use std::collections::hash_map::Entry::*;
        match self.return_slots.entry(request.request.clone()) {
            Occupied(slot) => {
                assert_ne!(slot.get().len(), 0);
                slot.into_mut().push(return_slot);
            }
            Vacant(slot) => {
                let inner_request = request.request.clone();
                let fut = self
                    .ctx
                    .clone()
                    .process_request(request)
                    .map(move |response| (inner_request, response));

                self.in_flight_futs.push(Box::pin(fut));

                slot.insert(vec![return_slot]);
            }
        }
    }
}

impl PostingContext {
    pub(crate) fn new(
        bot: tg::Bot,
        config: Arc<tg::Config>,
        params: posting::platform::PlatformParams<posting::all_platforms::Config>,
    ) -> Self {
        Self {
            bot,
            config,
            http: params.http.clone(),
            platforms: Arc::new(posting::AllPlatforms::new(params)),
        }
    }

    /// Combines both getting the post meta, and getting the cached blobs.
    ///
    /// Getting the post meta from the posting platform will dominate
    /// the time spent in this function, so reaching out to the
    /// cache almost doesn't influence the latency of the request.
    #[instrument(skip_all, fields(
        requested_by = %request.requested_by.debug_id(),
        ?request,
    ))]
    async fn process_request(self, request: CachePostRequest) -> Result<CachedPost> {
        let (post, cached_blobs) = futures::try_join!(
            self.platforms.get_post(request.request.clone()),
            self.platforms.get_cached_blobs(request.request)
        )?;

        let blobs: Vec<_> = post
            .blobs
            .iter()
            .filter_map(|blob| {
                let blob = cached_blobs
                    .iter()
                    .find(|cached_blob| cached_blob.id == blob.id)?;

                Some(blob.clone())
            })
            .collect();

        blob_cache_requests_total(vec![]).increment(1);

        if post.blobs.len() == blobs.len() {
            info!(blobs = blobs.len(), "Blobs cache hit");
            blob_cache_hits_total(vec![]).increment(1);
            return Ok(post.base.into_cached(request.mirror, blobs));
        }

        info!(
            matched = blobs.len(),
            actual = post.blobs.len(),
            cached = cached_blobs.len(),
            "Blobs cache miss"
        );

        let cached_blobs = stream::iter(post.blobs)
            .map(|blob| async {
                let cached_blob = tg_upload::upload(
                    &self,
                    request.mirror.as_ref(),
                    &post.base,
                    blob,
                    &request.requested_by,
                )
                .await?
                .to_id();

                let result = self
                    .platforms
                    .set_cached_blob(post.base.id.clone(), cached_blob.clone())
                    .await;

                if let Err(err) = result {
                    warn!(
                        err = tracing_err(&err),
                        "Failed to save cache info to the database"
                    );
                }

                Ok::<_, crate::Error>(cached_blob)
            })
            .buffer_unordered(10)
            .try_collect()
            .await?;

        Ok(post.base.into_cached(request.mirror, cached_blobs))
    }
}
