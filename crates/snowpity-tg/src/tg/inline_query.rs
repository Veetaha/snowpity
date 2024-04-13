use crate::posting::{self, TgFileKind};
use crate::prelude::*;
use crate::util::{encoding, DynResult};
use crate::{err, tg, Error, ErrorKind};
use futures::prelude::*;
use itertools::Itertools;
use metrics_bat::prelude::*;
use reqwest::Url;
use std::future::IntoFuture;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::{
    ChosenInlineResult, InlineQuery, InlineQueryResult, InlineQueryResultCachedDocument,
    InlineQueryResultCachedMpeg4Gif, InlineQueryResultCachedPhoto, InlineQueryResultCachedVideo,
    InlineQueryResultVideo, ParseMode,
};
use teloxide::utils::markdown;

const ERROR_VIDEO_URL: &str = "https://user-images.githubusercontent.com/36276403/209671572-9a3eada8-1bf6-4a9c-ac0e-44863f66746a.mp4";
const ERROR_VIDEO_THUMB_URL: &str = "https://user-images.githubusercontent.com/36276403/209673286-6cc10562-a5e1-4c90-b373-8290abd41fa7.jpg";

const CACHE_TIME_SECS: u32 = 0;

metrics_bat::labels! {
    InlineQueryTotalLabels { user }
    InlineQueryLabels { posting_platform_host }
}

metrics_bat::counters! {
    /// Number of inline queries received by the bot, but rejected due to parse errors
    inline_queries_skipped_total;

    /// Number of inline queries that were accepted
    chosen_inline_results_total;

    /// Number of inline queries taken into processing per user
    inline_queries_total;
}

metrics_bat::histograms! {
    /// Duration of a single inline query
    inline_query_duration_seconds = crate::metrics::DEFAULT_DURATION_BUCKETS;
}

pub(crate) struct InlineQueryService {
    posting: posting::PostingServiceHandle,
}

impl InlineQueryService {
    pub(crate) fn new(ctx: posting::PostingContext) -> Self {
        Self {
            posting: posting::spawn_service(ctx),
        }
    }
}

#[instrument(skip_all, fields(query = %query.query, from = %query.from.debug_id()))]
pub(crate) async fn handle(ctx: Arc<tg::Ctx>, query: InlineQuery) -> DynResult {
    let tg::Ctx {
        bot, inline_query, ..
    } = &*ctx;

    let inline_query_id = query.id;

    let Some((posting_platform_host, request_id)) = posting::parse_query(&query.query) else {
        inline_queries_skipped_total(vec![]).increment(1);

        info!("Skipping inline query");

        bot.answer_inline_query(inline_query_id, [])
            .switch_pm_text("Help")
            .switch_pm_parameter("help")
            .cache_time(CACHE_TIME_SECS)
            .await?;

        return Ok(());
    };

    let comments = query
        .query
        .lines()
        .skip(1)
        .skip_while(|line| line.is_empty())
        .join("\n");

    inline_queries_total(InlineQueryTotalLabels {
        user: query.from.debug_id(),
    })
    .increment(1);

    let labels = InlineQueryLabels {
        posting_platform_host: posting_platform_host.clone(),
    };

    async {
        let request = posting::CachePostRequest {
            requested_by: query.from,
            id: request_id,
        };

        let post = inline_query.posting.cache_post(request).await?;

        if post.blobs.is_empty() {
            return Err(err!(InlineQueryError::MissingMedia));
        }

        let tg_file_types = post
            .blobs
            .iter()
            .map(|response| response.tg_file.kind)
            .unique()
            .join(", ");

        let total_blobs = post.blobs.len();

        let results = post
            .blobs
            .into_iter()
            .map(|blob| make_inline_query_result(&comments, &post.base, blob));

        bot.answer_inline_query(&inline_query_id, results)
            .is_personal(false)
            .cache_time(CACHE_TIME_SECS)
            .into_future()
            .with_duration_log("Answering inline query")
            .instrument(info_span!("payload", %tg_file_types, total_blobs))
            .await?;

        Ok::<_, Error>(())
    }
    .with_duration_log("Processed inline query")
    .record_duration(inline_query_duration_seconds, labels)
    .or_else(|err| async {
        // The title is very constrained in size. We must be very succinct in it.
        let default_title = "Something went wrong 🥺";
        let title_suffix = ". Try another link 😅";

        let (title, full_err) = match err.kind() {
            ErrorKind::Posting { source } => (source.to_string(), "".to_owned()),
            _ => (
                default_title.to_owned(),
                format!(
                    "\n\n{}",
                    markdown::code_block(&err.display_chain().to_string())
                ),
            ),
        };

        let title = title + title_suffix;

        let link = query.query.trim();

        let caption = format!(
            "*{}*\n\nLink: {}{}",
            markdown::escape(&title),
            markdown::escape(link),
            full_err
        );

        let video_url: Url = ERROR_VIDEO_URL.parse().unwrap();
        let video_thumb_url: Url = ERROR_VIDEO_THUMB_URL.parse().unwrap();

        let result = InlineQueryResultVideo::new(
            err.id(),
            video_url,
            "video/mp4".parse().unwrap(),
            video_thumb_url,
            title,
        )
        .caption(caption)
        .parse_mode(ParseMode::MarkdownV2)
        .into();

        let result = bot
            .answer_inline_query(&inline_query_id, [result])
            .is_personal(false)
            .cache_time(0)
            .into_future()
            .await;

        if let Err(err) = result {
            warn!(
                err = tracing_err(&err),
                "Failed to answer with error to inline query"
            );
        }

        Err(err)
    })
    .err_into()
    .await
}

fn make_inline_query_result(
    comments: &str,
    post: &posting::BasePost,
    blob: posting::CachedBlobId,
) -> InlineQueryResult {
    let mut caption = post.caption();
    if !comments.is_empty() {
        caption = format!("{caption}\n\n{}", markdown::escape(comments));
    }

    let parse_mode = ParseMode::MarkdownV2;
    let title = "Click to send";
    let id = encoding::encode_base64_sha2(&blob.tg_file.id);
    let file_id = blob.tg_file.id;

    match blob.tg_file.kind {
        TgFileKind::Photo => InlineQueryResultCachedPhoto::new(id, file_id)
            .caption(caption)
            // XXX: title is ignored for photos in results preview popup.
            // That's really surprising, but that's how telegram works -_-
            .title(title)
            .parse_mode(parse_mode)
            .into(),
        TgFileKind::Document => InlineQueryResultCachedDocument::new(id, title, file_id)
            .caption(caption)
            .title(title)
            .parse_mode(parse_mode)
            .into(),
        TgFileKind::Video => InlineQueryResultCachedVideo::new(id, file_id, title)
            .caption(caption)
            .title(title)
            .parse_mode(parse_mode)
            .into(),
        TgFileKind::Mpeg4Gif => InlineQueryResultCachedMpeg4Gif::new(id, file_id)
            .caption(caption)
            // XXX: title is ignored for gifs as well as for photos,
            // see the comment on photos match arm above
            .title(title)
            .parse_mode(parse_mode)
            .into(),
    }
}

/// XXX: This handler must be enabled manually via `/setinlinefeedback` command in
/// Telegram BotFather, otherwise `ChosenInlineResult` updates will not be sent.
pub(crate) async fn handle_chosen_inline_result(result: ChosenInlineResult) -> DynResult {
    let posting_platform_host = posting::parse_query(&result.query)
        .map(|(host, _id)| host)
        .unwrap_or("{unknown}".to_owned());

    let labels = InlineQueryLabels {
        posting_platform_host,
    };

    chosen_inline_results_total(labels).increment(1);

    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum InlineQueryError {
    #[error("The post contains no media")]
    MissingMedia,
}
