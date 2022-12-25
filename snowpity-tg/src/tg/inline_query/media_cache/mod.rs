mod derpi_cache;

use crate::db::CachedMedia;
use crate::prelude::*;
use crate::util::{self, http};
use crate::{db, derpi, tg, Result};
use futures::future::BoxFuture;
use futures::prelude::*;
use futures::stream::FuturesUnordered;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};

pub(crate) use derpi_cache::*;

metrics_bat::gauges! {
    /// Number of in-flight requests for media cache
    media_cache_requests_in_flight_total;
}

/// Maximum number of in-flight cache requests, otherwise the service will
/// block the new requests.
const MAX_IN_FLIGHT: usize = 40;
const UNEXPECTED_SERVICE_SHUTDOWN: &str = "BUG: Service exited unexpectedly";

pub(crate) type DerpiEnvelope = Envelope<DerpiRequest>;
pub(crate) type DerpiRequest = derpi_cache::Request;

/// Determines the API method used when the media was uploaded to Telegram.
#[derive(
    Clone,
    Copy,
    Debug,
    IntoPrimitive,
    TryFromPrimitive,
    strum::Display,
    strum::IntoStaticStr,
    sqlx::Type,
)]
#[repr(i16)]
pub(crate) enum TgFileType {
    Photo = 0,
    Document = 1,
    Video = 2,
    Gif = 3,
    // Mpeg4Gif = 4,
}

sqlx_bat::impl_try_into_from_db_via_std!(TgFileType, i16);

pub(crate) struct Envelope<R> {
    request: R,
    return_slot: oneshot::Sender<Result<Response>>,
}

impl<P: fmt::Debug> fmt::Debug for Envelope<P> {
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

#[derive(Clone)]
pub(crate) struct Response {
    pub(crate) media: derpi::Media,
    pub(crate) cached: CachedMedia,
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
    send: Option<mpsc::Sender<DerpiEnvelope>>,
    join_handle: Option<tokio::task::JoinHandle<()>>,
}

#[derive(Clone)]
pub(crate) struct Context {
    pub(crate) bot: tg::Bot,
    pub(crate) derpi: Arc<derpi::DerpiService>,
    pub(crate) cfg: Arc<tg::Config>,
    pub(crate) db: Arc<db::Repo>,
    pub(crate) http_client: http::Client,
}

struct Service {
    ctx: Context,

    in_flight_futs: FuturesUnordered<BoxFuture<'static, (derpi::MediaId, Result<Response>)>>,
    return_slots: HashMap<derpi::MediaId, Vec<oneshot::Sender<Result<Response>>>>,
    requests: mpsc::Receiver<DerpiEnvelope>,
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
    pub(crate) async fn get_tg_derpi_media(&self, request: DerpiRequest) -> Result<Response> {
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

impl<P> Envelope<P> {
    fn new(payload: P) -> (Self, impl Future<Output = Result<Response>>) {
        let (send, recv) = oneshot::channel();
        let me = Self {
            request: payload,
            return_slot: send,
        };
        (me, recv.map(|val| val.expect(UNEXPECTED_SERVICE_SHUTDOWN)))
    }
}

impl Service {
    #[instrument(skip(self))]
    async fn run_loop(mut self) {
        loop {
            let total_in_flight = self.total_in_flight();
            media_cache_requests_in_flight_total(vec![]).set(total_in_flight as f64);

            tokio::select! {
                // This `if` condition implements a simple backpressure mechanism
                // to prevent receiving new requests when the number of in-flight
                // requests is too high.
                request = self.requests.recv(), if total_in_flight <= MAX_IN_FLIGHT => {
                    let Some(request) = request else {
                        info!("Channel closed, exiting...");
                        return;
                    };
                    self.process_request(request);
                }
                Some((media_id, response)) = self.in_flight_futs.next() => {
                    self.dispatch_response(media_id, response);
                }
            }
        }
    }

    fn total_in_flight(&self) -> usize {
        self.return_slots
            .values()
            .map(|res| res.len())
            .sum::<usize>()
    }

    #[instrument(skip(self, response))]
    fn dispatch_response(&mut self, media_id: derpi::MediaId, response: Result<Response>) {
        let slots = self
            .return_slots
            .remove(&media_id)
            .expect("BUG: an in-flight future must have a corresponding response return slot");

        for slot in slots {
            if slot.send(response.clone()).is_err() {
                warn!("Failed to send response because the receiver has been dropped");
            }
        }
    }

    #[instrument(skip(self))]
    fn process_request(&mut self, request: Envelope<DerpiRequest>) {
        let Envelope {
            request,
            return_slot,
        } = request;

        let media_id = request.media_id;

        use std::collections::hash_map::Entry::*;
        match self.return_slots.entry(media_id) {
            Occupied(slot) => {
                assert_ne!(slot.get().len(), 0);
                slot.into_mut().push(return_slot);
            }
            Vacant(slot) => {
                let fut = derpi_cache::cache(self.ctx.clone(), request)
                    .map(move |response| (media_id, response));

                self.in_flight_futs.push(Box::pin(fut));

                slot.insert(vec![return_slot]);
            }
        }
    }
}
