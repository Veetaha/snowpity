use crate::util::prelude::*;
use crate::util::{DynResult, TgFileType};
use crate::{derpi, tg, Error};
use futures::prelude::*;
use lazy_regex::regex_captures;
use metrics_bat::prelude::*;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::{
    ChosenInlineResult, InlineQuery, InlineQueryResultCachedDocument, InlineQueryResultCachedPhoto,
    InlineQueryResultCachedVideo, ParseMode,
};

pub(crate) mod media_cache;

metrics_bat::labels! {
    InlineQueryTotalLabels { user }
    InlineQuerySkippedLabels { }
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

#[instrument(skip_all, fields(query = %query.query))]
pub(crate) async fn handle_inline_query(ctx: Arc<tg::Ctx>, query: InlineQuery) -> DynResult {
    let tg::Ctx {
        bot, inline_query, ..
    } = &*ctx;

    let inline_query_id = query.id;

    let Some((media_host, media_id)) = parse_query(&query.query) else {
        inline_queries_skipped_total(InlineQuerySkippedLabels {}).increment(1);
        return Ok(());
    };

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
            media_id,
        };

        let media_cache::Response { media, cached } = inline_query
            .media_cache_client
            .get_tg_derpi_media(payload)
            .await?;

        // FIXME: ensure the caption doesn't overflow 1024 characters
        let caption = media_cache::core_caption(&media);

        let file_name = media_cache::file_name(&media);

        let result = match cached.tg_file_type {
            TgFileType::Photo => {
                info!("Returning image for inline query");

                InlineQueryResultCachedPhoto::new(media_id.to_string(), cached.tg_file_id)
                    .caption(caption)
                    .parse_mode(ParseMode::MarkdownV2)
                    .into()
            }
            TgFileType::Document => {
                info!("Returning document for inline query");

                InlineQueryResultCachedDocument::new(
                    media_id.to_string(),
                    file_name,
                    cached.tg_file_id,
                )
                .caption(caption)
                .parse_mode(ParseMode::MarkdownV2)
                .into()
            }
            TgFileType::Video => {
                info!("Returning video for inline query");

                InlineQueryResultCachedVideo::new(
                    media_id.to_string(),
                    cached.tg_file_id,
                    file_name,
                )
                .caption(caption)
                .parse_mode(ParseMode::MarkdownV2)
                .into()
            }
        };

        bot.answer_inline_query(inline_query_id, [result])
            .is_personal(false)
            .cache_time(u32::MAX)
            .await?;

        Ok::<_, Error>(())
    }
    .record_duration(inline_query_duration_seconds, labels)
    .err_into()
    .await
}

fn parse_query(str: &str) -> Option<(&str, derpi::MediaId)> {
    let str = str.trim();
    let (_, host, id) = regex_captures!("(derpibooru.org/images)/(\\d+)", str)
        .or_else(|| regex_captures!("(derpicdn.net/img)/\\d+/\\d+/\\d+/(\\d+)", str))
        .or_else(|| regex_captures!("(derpicdn.net/img/view)/\\d+/\\d+/\\d+/(\\d+)", str))?;
    Some((host, id.parse().ok()?))
}

pub(crate) async fn handle_chosen_inline_result(result: ChosenInlineResult) -> DynResult {
    let media_host = parse_query(&result.query)
        .map(|(host, _id)| host)
        .unwrap_or("{unknown}");

    chosen_inline_results_total(InlineQueryLabels {
        user: result.from.debug_id(),
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
    }
}
