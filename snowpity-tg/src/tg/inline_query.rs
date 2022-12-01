use crate::derpi::rpc::MimeType;
use crate::util::prelude::*;
use crate::{db, derpi, tg, DynResult, Error, Result};
use futures::channel::oneshot;
use futures::prelude::*;
use lazy_regex::regex_captures;
use std::collections::HashMap;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::{ChatId, InlineQueryResultCachedPhoto, InputFile};
use teloxide::types::{InlineQuery, ParseMode};
use teloxide::utils::markdown;
use tokio::sync::{mpsc, OnceCell as AsyncOnceCell};

pub(crate) struct InlineQueryService {
    derpi_media_cache: Actor<DerpiMediaCacheService>,
}

impl InlineQueryService {
    pub(crate) fn new(
        bot: tg::Bot,
        derpi: Arc<derpi::DerpiService>,
        cfg: Arc<tg::Config>,
        db: Arc<db::Repo>,
    ) -> Self {
        let service = DerpiMediaCacheService {
            bot,
            derpi,
            cfg,
            db,
            in_flight_requests: Default::default(),
        };

        let (actor, driver) = Actor::new_with_capacity(service, 40);
        tokio::spawn(driver);

        Self {
            derpi_media_cache: actor,
        }
    }

    async fn cache_derpi_media(&self, media_id: u64) -> Result<String> {
        let res = self
            .derpi_media_cache
            .query_blocking(|service| {
                Box::pin(async {

                    // service.in_flight_requests.get()

                    // 32
                })
            })
            .await
            .unwrap();

        // let (tx, rx) = oneshot::channel();
        // self.derpi_media_cache
        //     .send(DerpiMediaCacheRequest { media_id, return_slot })
        //     .await?;
        // rx.await?
        Ok(todo!())
    }
}

pub(crate) async fn handle_inline_query(ctx: Arc<tg::Ctx>, query: InlineQuery) -> DynResult {
    async {
        let tg::Ctx {
            bot,
            db,
            inline_query,
            ..
        } = &*ctx;

        let inline_query_id = query.id;
        let query = query.query;

        let Some((_, media_id)) = regex_captures!(r"derpibooru.org/images/(\d+)", &query) else {
            return Ok(());
        };
        let Ok(media_id) = media_id.parse() else {
            return Ok(());
        };

        let tg_file_id = db.media_cache.get_derpi_tg_file_id(media_id).await?;

        // let tg_file_id = match tg_file_id {
        //     Some(cached) => cached,
        //     None => {
        //         // inline_query.
        //         // cache_derpi_media(&ctx, media_id).await?,
        //     }
        // };

        // let image = derpi.get_media(media_id).await?;

        // if image.mime_type != ImageMimeType::ImageJpeg && image.mime_type != ImageMimeType::ImagePng
        // {
        //     return Ok(());
        // }

        // info!(
        //     "Sending inline query result for image {}",
        //     image.representations.full
        // );

        // let caption = markdown::link(
        //     &String::from(derpibooru::media_id_to_webpage_url(media_id)),
        //     r"_*Source link \(derpibooru\.org\)*_",
        // );

        // let input_file = InputFile::url(image.representations.full);

        // let msg = ctx
        //     .bot
        //     .send_photo(*media_cache_chat_id, input_file)
        //     .caption(caption.clone())
        //     .await?;

        // let photo = &msg.photo().unwrap()[0].file;

        // dbg!(&photo);

        // if image.size > 5000 {
        //     return Ok(());
        // }

        // ctx.bot.send_photo(
        //     chat_id,
        //     photo
        // );

        // let result = InlineQueryResultCachedPhoto::new(media_id.to_string(), photo.id.clone())
        //     .caption(caption)
        //     .parse_mode(ParseMode::MarkdownV2)
        //     .into();

        // bot.answer_inline_query(inline_query_id, [result])
        //     .is_personal(false)
        //     .cache_time(u32::MAX)
        //     .await?;

        Ok::<_, Error>(())
    }
    .err_into()
    .await
}

struct DerpiMediaCacheRequest {
    media_id: u64,
    return_slot: oneshot::Sender<Result<String>>,
}

async fn spawn_derpi_media_cache_service(
    ctx: Arc<tg::Ctx>,
) -> (
    mpsc::Sender<DerpiMediaCacheRequest>,
    tokio::task::JoinHandle<()>,
) {
    todo!()
}

struct DerpiMediaCacheService {
    bot: tg::Bot,
    derpi: Arc<derpi::DerpiService>,
    cfg: Arc<tg::Config>,
    db: Arc<db::Repo>,
    in_flight_requests: HashMap<u64, Vec<oneshot::Sender<String>>>,
}

impl DerpiMediaCacheService {
    async fn run_loop(self) {

        // let Self {
        //     bot,
        //     derpi,
        //     cfg,
        //     db,
        //     in_flight_requests: mut pending_requests,
        //     rx,
        // } = self;

        // let lock = Arc::new(AsyncOnceCell::new());

        // inline_query
        //     .pending_queries
        //     .lock()
        //     .insert(media_id, lock.clone());

        // let media = derpi.get_media(media_id).await?;

        // db.media_cache
        //     .set_derpi_tg_file_id(media_id_str, &tg_file_id)
        //     .await?;

        // Ok(tg_file_id)
    }
}
