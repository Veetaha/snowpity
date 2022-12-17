use crate::metrics::def_metrics;
use crate::util::prelude::*;
use crate::util::{DynResult, TgFileType};
use crate::{derpi, tg, Error};
use futures::prelude::*;
use lazy_regex::regex_captures;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::{
    InlineQuery, InlineQueryResultCachedDocument, InlineQueryResultCachedPhoto,
    InlineQueryResultCachedVideo, ParseMode,
};

pub(crate) mod media_cache;

def_metrics! {
    /// Number of inline queries received by the bot
    inline_queries: IntCounter;

    /// Number of errors while handling inline queries
    inline_queries_errors: IntCounter;

    /// Number of inline queries which were accepted
    chosen_inline_results: IntCounter;
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
    async {
        inline_queries().inc();

        let tg::Ctx {
            bot, inline_query, ..
        } = &*ctx;

        let inline_query_id = query.id;

        let Some(media_id) = parse_query(&query.query) else {
            return Ok(());
        };

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
    .inspect_err(|_| inline_queries_errors().inc())
    .err_into()
    .await
}

fn parse_query(str: &str) -> Option<derpi::MediaId> {
    // FIXME: support all possible derpicdn URLs
    let (_, booru, cdn_repr, cdn_view) = regex_captures!(
        "(?:derpibooru.org/images/(\\d+))\
        |(?:derpicdn.net/img/\\d+/\\d+/\\d+/(\\d+))
        |(?:derpicdn.net/img/view/\\d+/\\d+/\\d+/(\\d+))",
        str.trim()
    )?;
    [booru, cdn_repr, cdn_view]
        .iter()
        .find(|capture| !capture.is_empty())?
        .parse()
        .ok()
}

pub(crate) async fn handle_chosen_inline_result() -> DynResult {
    chosen_inline_results().inc();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::{expect, Expect};

    #[track_caller]
    fn assert_parse_query(query: &str, expected: Expect) {
        let actual = if let Some(id) = parse_query(query) {
            id.to_string()
        } else {
            "None".to_string()
        };
        expected.assert_eq(&actual);
    }

    #[test]
    fn query_parsing() {
        use assert_parse_query as test;
        test("123", expect!["None"]);
        test("derpibooru.org/images/123", expect!["123"]);
        test("derpibooru.org/images/123/", expect!["123"]);
        test("furbooru.org/images/123/", expect!["None"]);
    }
}
