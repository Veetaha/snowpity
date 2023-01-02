use super::{imp, tg_upload, CachedMedia, MediaId, MediaMeta, TgFileMeta};
use crate::media_host::{self, derpi, twitter};
use crate::prelude::*;
use crate::{db, http, tg, util, Result};
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
    /// Number of in-flight requests for media cache
    media_cache_requests_in_flight_total;
}

metrics_bat::counters! {
    /// Number of times we hit the database cache for derpibooru media
    media_cache_hits_total;

    /// Number of times we queried the database cache for derpibooru media
    media_cache_requests_total;
}

pub(crate) struct Envelope {
    request: Request,
    return_slot: oneshot::Sender<Result<Response>>,
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

#[derive(Debug)]
pub(crate) struct Request {
    pub(crate) requested_by: teloxide::types::User,
    pub(crate) id: RequestId,
}

#[derive(Clone)]
pub(crate) struct Response {
    pub(crate) items: Vec<ResponseItem>,
}

#[derive(Clone)]
pub(crate) struct ResponseItem {
    pub(crate) tg_file: TgFileMeta,
    pub(crate) media_meta: MediaMeta,
}

impl ResponseItem {
    fn new(tg_file: TgFileMeta, media_meta: MediaMeta) -> Self {
        Self {
            tg_file,
            media_meta,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, from_variants::FromVariants)]
pub(crate) enum RequestId {
    Derpibooru(derpi::MediaId),
    Twitter(twitter::TweetId),
}

pub(crate) fn spawn_service(ctx: Context) -> Client {
    let (send, recv) = mpsc::channel(MAX_IN_FLIGHT);
    let service = Service {
        ctx,
        in_flight_futs: Default::default(),
        return_slots: Default::default(),
        requests: recv,
    };
    Client {
        send: Some(send),
        join_handle: Some(tokio::spawn(service.run_loop())),
    }
}

pub(crate) struct Client {
    send: Option<mpsc::Sender<Envelope>>,
    join_handle: Option<tokio::task::JoinHandle<()>>,
}

#[derive(Clone)]
pub(crate) struct Context {
    pub(crate) bot: tg::Bot,
    pub(crate) media: Arc<media_host::Client>,
    pub(crate) cfg: Arc<tg::Config>,
    pub(crate) db: Arc<db::Repo>,
    pub(crate) http: http::Client,
}

struct Service {
    ctx: Context,

    in_flight_futs: FuturesUnordered<BoxFuture<'static, (RequestId, Result<Response>)>>,
    return_slots: HashMap<RequestId, Vec<oneshot::Sender<Result<Response>>>>,
    requests: mpsc::Receiver<Envelope>,
}

impl Client {
    /// Returns the telegram file id for the given Derpibooru media id.
    /// It maintains a cache of media, that was already requested, using
    /// a database, and saving the files in a dedicated telegram channel,
    /// if the media is requested for the first time.
    ///
    /// It's totally fine to call this method concurrently and with the same
    /// `media_id` repeatedly, but there is a backpressure mechanism so that
    /// the future won't resolve until the service's capacity is available.
    pub(crate) async fn get_media(&self, request: Request) -> Result<Response> {
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

impl Drop for Client {
    fn drop(&mut self) {
        // Drop the sender to signal the service to exit.
        self.send = None;
        util::block_in_place(self.join_handle.take().unwrap());
    }
}

impl Envelope {
    fn new(request: Request) -> (Self, impl Future<Output = Result<Response>>) {
        let (send, recv) = oneshot::channel();
        let me = Self {
            request,
            return_slot: send,
        };
        (me, recv.map(|val| val.expect(UNEXPECTED_SERVICE_SHUTDOWN)))
    }
}

impl Service {
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
        media_cache_requests_in_flight_total(vec![]).set(total_in_flight as f64);

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
    fn dispatch_response(&mut self, request_id: RequestId, response: Result<Response>) {
        let slots = self
            .return_slots
            .remove(&request_id)
            .expect("BUG: an in-flight future must have a corresponding response return slot");

        for slot in slots {
            if slot.send(response.clone()).is_err() {
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

        use std::collections::hash_map::Entry::*;
        match self.return_slots.entry(request.id.clone()) {
            Occupied(slot) => {
                assert_ne!(slot.get().len(), 0);
                slot.into_mut().push(return_slot);
            }
            Vacant(slot) => {
                let request_id = request.id.clone();

                let fut = self
                    .ctx
                    .clone()
                    .process_request(request)
                    .map(move |response| (request_id, response));

                self.in_flight_futs.push(Box::pin(fut));

                slot.insert(vec![return_slot]);
            }
        }
    }
}

impl Context {
    #[instrument(skip_all, fields(
        requested_by = %request.requested_by.debug_id(),
        request_id = ?request.id,
    ))]
    async fn process_request(self, request: Request) -> Result<Response> {
        let (media_meta, cached_media) = self.resolve_media(&request).await?;

        let items: Vec<_> = media_meta
            .iter()
            .filter_map(|meta| {
                let cached = cached_media.iter().find(|cached| cached.id == meta.id)?;
                Some(ResponseItem::new(cached.tg_file.clone(), meta.clone()))
            })
            .collect();

        media_cache_requests_total(vec![]).increment(1);

        if !items.is_empty() && items.len() == cached_media.len() {
            info!(items = items.len(), "Media cache hit");
            media_cache_hits_total(vec![]).increment(1);
            return Ok(Response { items });
        }

        info!(
            matched = items.len(),
            media = media_meta.len(),
            cached = cached_media.len(),
            "Media cache miss"
        );

        let items = stream::iter(media_meta)
            .map(|meta| async {
                let tg_file = tg_upload::upload(&self, &meta, &request.requested_by).await?;

                if let Err(err) = self.set_cache(&meta, &tg_file).await {
                    warn!(
                        err = tracing_err(&err),
                        "Failed to save cache info in the database"
                    );
                }

                Ok::<_, crate::Error>(ResponseItem::new(tg_file, meta))
            })
            .buffer_unordered(10)
            .try_collect()
            .await?;

        Ok(Response { items })
    }

    /// Fetch metadata about the media from the media hosting, and the cached
    /// version of the media from the database. Talking to the media hosting
    /// will dominate the time spent in this function, so reaching out to the
    /// cache almost doesn't influence the latency of the request.
    async fn resolve_media(&self, request: &Request) -> Result<(Vec<MediaMeta>, Vec<CachedMedia>)> {
        match request.id {
            RequestId::Derpibooru(media_id) => {
                let (media_meta, cached_media) = futures::try_join!(
                    imp::derpi::get_media_meta(self, media_id),
                    imp::derpi::get_cached_media(self, media_id),
                )?;

                Ok((vec![media_meta], Vec::from_iter(cached_media)))
            }
            RequestId::Twitter(tweet_id) => futures::try_join!(
                imp::twitter::get_media_meta(self, tweet_id),
                imp::twitter::get_cached_media(self, tweet_id),
            ),
        }
    }

    /// Save the information about the file uploaded to Telegram in the database.
    async fn set_cache(&self, media: &MediaMeta, tg_file: &TgFileMeta) -> Result {
        let tg_file = tg_file.clone();
        match &media.id {
            MediaId::Derpibooru(media_id) => {
                self.db.tg_media_cache.derpi.set(*media_id, tg_file).await
            }
            MediaId::Twitter(tweet_id, media_key) => {
                self.db
                    .tg_media_cache
                    .twitter
                    .set(*tweet_id, media_key.clone(), tg_file)
                    .await
            }
        }
    }
}
