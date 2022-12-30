use super::{media_cache, MediaCacheError, TgFileKind};
use crate::media_host::derpi;
use crate::prelude::*;
use crate::util::DynResult;
use crate::{encoding, tg, Error, ErrorKind};
use futures::prelude::*;
use lazy_regex::regex_captures;
use metrics_bat::prelude::*;
use reqwest::Url;
use std::future::IntoFuture;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::{
    ChosenInlineResult, InlineQuery, InlineQueryResultCachedDocument,
    InlineQueryResultCachedMpeg4Gif, InlineQueryResultCachedPhoto, InlineQueryResultCachedVideo,
    InlineQueryResultVideo, ParseMode,
};
use teloxide::utils::markdown;

const ERROR_VIDEO_URL: &str = "https://user-images.githubusercontent.com/36276403/209671572-9a3eada8-1bf6-4a9c-ac0e-44863f66746a.mp4";
const ERROR_VIDEO_THUMB_URL: &str = "https://user-images.githubusercontent.com/36276403/209673286-6cc10562-a5e1-4c90-b373-8290abd41fa7.jpg";

metrics_bat::labels! {
    InlineQueryTotalLabels { user }
    InlineQueryLabels { media_host }
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
    media_cache_client: media_cache::Client,
}

impl InlineQueryService {
    pub(crate) fn new(ctx: media_cache::Context) -> Self {
        Self {
            media_cache_client: media_cache::spawn_service(ctx),
        }
    }
}

#[instrument(skip_all, fields(query = %query.query, from = %query.from.debug_id()))]
pub(crate) async fn handle(ctx: Arc<tg::Ctx>, query: InlineQuery) -> DynResult {
    let tg::Ctx {
        bot, inline_query, ..
    } = &*ctx;

    let inline_query_id = query.id;

    let Some((media_host, media_id)) = parse_query(&query.query) else {
        inline_queries_skipped_total(vec![]).increment(1);
        info!("Skipping inline query");
        return Ok(());
    };

    info!("Processing inline query");

    inline_queries_total(InlineQueryTotalLabels {
        user: query.from.debug_id(),
    })
    .increment(1);

    let labels = InlineQueryLabels {
        media_host: media_host.to_owned(),
    };

    async {
        let payload = media_cache::DerpiRequest {
            requested_by: query.from,
            request_id,
        };

        let response = inline_query
            .media_cache_client
            .get_tg_derpi_media(payload)
            .await?;

        // FIXME: ensure the caption doesn't overflow 1024 characters
        let caption = response.meta.caption();

        let parse_mode = ParseMode::MarkdownV2;
        let title = "Click to send";
        let id = encoding::encode_base64_sha2(&response.tg_file_id);

        let result = match response.tg_file_type {
            TgFileKind::Photo => InlineQueryResultCachedPhoto::new(id, response.tg_file_id)
                .caption(caption)
                .parse_mode(parse_mode)
                // XXX: title is ignored for photos in when telegram clients display the results.
                // That's really surprising, but that's how telegram works -_-
                .title(title)
                .into(),
            TgFileKind::Document => {
                InlineQueryResultCachedDocument::new(id, title, response.tg_file_id)
                    .caption(caption)
                    .parse_mode(parse_mode)
                    .into()
            }
            TgFileKind::Video => InlineQueryResultCachedVideo::new(id, response.tg_file_id, title)
                .caption(caption)
                .parse_mode(parse_mode)
                .into(),
            TgFileKind::Mpeg4Gif => InlineQueryResultCachedMpeg4Gif::new(id, response.tg_file_id)
                .caption(caption)
                // XXX: title is ignored for gifs as well as for photos,
                // see the comment on photos match arm above
                .title(title)
                .parse_mode(parse_mode)
                .into(),
        };

        bot.answer_inline_query(&inline_query_id, [result])
            .is_personal(false)
            .cache_time(u32::MAX)
            .into_future()
            .with_duration_log("Answering inline query")
            .instrument(info_span!("payload", tg_file_type = %response.tg_file_type))
            .await?;

        Ok::<_, Error>(())
    }
    .record_duration(inline_query_duration_seconds, labels)
    .or_else(|err| async {
        // The title is very constrained in size. We must be very succinct in it.
        let default_title = "Something went wrong 🥺";

        let title_prefix = match err.kind() {
            ErrorKind::MediaCache { source } => match source {
                MediaCacheError::FileTooBig { actual, max } => {
                    let mem = humansize::make_format(humansize::BINARY);
                    let actual = mem(*actual);
                    let max = mem(*max);

                    format!("Too big ({actual}, max: {max})")
                }
                _ => default_title.to_owned(),
            },
            _ => default_title.to_owned(),
        };

        let title = format!("{title_prefix}. Try another image 😅");

        let caption = format!(
            "*{}*\n\n{}",
            markdown::escape(&title),
            markdown::code_block(&err.display_chain().to_string())
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
            .is_personal(true)
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

fn parse_query(str: &str) -> Option<(&str, derpi::MediaId)> {
    let str = str.trim();
    let (_, host, id) = None
        .or_else(|| regex_captures!("(derpibooru.org(?:/images)?)/(\\d+)", str))
        .or_else(|| regex_captures!("(derpicdn.net/img)/\\d+/\\d+/\\d+/(\\d+)", str))
        .or_else(|| {
            regex_captures!(
                "(derpicdn.net/img/(?:view|download))/\\d+/\\d+/\\d+/(\\d+)",
                str
            )
        })?;
    Some((host, id.parse().ok()?))
}

/// XXX: This handler must be enabled manually via `/setinlinefeedback` command in
/// Telegram BotFather, otherwise `ChosenInlineResult` updates will not be sent.
pub(crate) async fn handle_chosen_inline_result(result: ChosenInlineResult) -> DynResult {
    let media_host = parse_query(&result.query)
        .map(|(host, _id)| host)
        .unwrap_or("{unknown}");

    chosen_inline_results_total(InlineQueryLabels {
        media_host: media_host.to_owned(),
    })
    .increment(1);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::{expect, Expect};

    #[track_caller]
    fn assert_parse_query(query: &str, expected: Expect) {
        let actual = if let Some((media_host, id)) = parse_query(query) {
            format!("{media_host}:{id}")
        } else {
            "None".to_owned()
        };
        expected.assert_eq(&actual);
    }

    #[test]
    fn query_parsing() {
        use assert_parse_query as test;
        test("123", expect!["None"]);
        test("furbooru.org/images/123/", expect!["None"]);

        test("derpibooru.org/123/", expect!["derpibooru.org:123"]);
        test("derpibooru.org/123", expect!["derpibooru.org:123"]);
        test(
            "derpibooru.org/images/123",
            expect!["derpibooru.org/images:123"],
        );
        test(
            "derpibooru.org/images/123/",
            expect!["derpibooru.org/images:123"],
        );
        test(
            "https://derpicdn.net/img/2022/12/17/3008328/large.jpg",
            expect!["derpicdn.net/img:3008328"],
        );
        test(
            "https://derpicdn.net/img/view/2022/12/17/3008328.jpg",
            expect!["derpicdn.net/img/view:3008328"],
        );
        test(
            "https://derpicdn.net/img/download/2022/12/28/3015836__safe_artist-colon-shadowreindeer_foo.jpg",
            expect!["derpicdn.net/img/download:3015836"]
        );
    }
}
