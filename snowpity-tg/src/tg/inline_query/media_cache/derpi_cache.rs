use super::{Context, Response};
use crate::util::http;
use crate::util::prelude::*;
use crate::{derpi, err_val, tg, util, ErrorKind, MediaError, Result};
use assert_matches::assert_matches;
use fs_err::tokio as fs;
use itertools::Itertools;
use reqwest::Url;
use std::fmt;
use std::path::PathBuf;
use std::time::Instant;
use teloxide::prelude::*;
use teloxide::types::{FileMeta, InputFile, MessageKind, User};
use teloxide::utils::markdown;

const MB: u64 = 1024 * 1024;

#[derive(Debug)]
pub(crate) struct Request {
    pub(crate) requested_by: User,
    pub(crate) media_id: derpi::MediaId,
}

#[instrument(skip_all, fields(
    requested_by = %payload.requested_by.debug_id(),
    media_id = %payload.media_id,
))]
pub(crate) async fn cache(ctx: Context, payload: Request) -> Result<Response> {
    // It's very likely neither of the requests will fail, so we
    // optimistically do them concurrently
    let (media, cached) = futures::try_join!(
        async {
            let start = std::time::Instant::now();
            let media = ctx.derpi.get_media(payload.media_id).await;
            info!(
                duration = format_args!("{:.2?}", start.elapsed()),
                "Fetched media meta from Derpibooru"
            );
            media
        },
        async {
            let start = std::time::Instant::now();
            let cached = ctx.db.media_cache.get_from_derpi(payload.media_id).await;
            info!(
                duration = format_args!("{:.2?}", start.elapsed()),
                "Read the cache from the database"
            );
            cached
        }
    )?;

    if let Some(cached) = cached {
        info!("Returning media from cache");
        return Ok(Response {
            media,
            tg_file_id: cached.tg_file_id,
        });
    }

    // FIXME: add metrics gatherting
    let file = TgUploadContext {
        base: &ctx,
        payload: &payload,
        media: &media,
    }
    .upload()
    .await?;

    let cached = ctx.db.media_cache.set_derpi(media.id, &file.id).await?;

    Ok(Response {
        media,
        tg_file_id: cached.tg_file_id,
    })
}

enum InputFileKind {
    /// The URL will be directly forwarded to telegram.
    Url(Url),
    /// We'll download the content ourselves and upload it to telegram using
    /// [`InputFile::memory`] kind. This is useful when the size of the file
    /// exceeds the limits for direct URL uploads.
    DownloadedUrl(Url),
    File(PathBuf),
}

impl fmt::Debug for InputFileKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Url(url) => f.debug_tuple("Url").field(&format_args!("{url}")).finish(),
            Self::DownloadedUrl(url) => f
                .debug_tuple("DownloadedUrl")
                .field(&format_args!("{url}"))
                .finish(),
            Self::File(path) => f.debug_tuple("File").field(path).finish(),
        }
    }
}

impl InputFileKind {
    async fn into_input_file(self, http_client: &http::Client) -> Result<InputFile> {
        Ok(match self {
            Self::Url(url) => InputFile::url(url),
            Self::File(path) => InputFile::file(path),
            Self::DownloadedUrl(url) => {
                let start = Instant::now();
                let bytes = http_client.get(url).read_bytes().await?;
                let elapsed = start.elapsed();
                let actual_size = bytes.len();
                let file = InputFile::memory(bytes);
                info!(
                    %actual_size,
                    took = format_args!("{elapsed:.2?}"),
                    "Downloaded file"
                );
                file
            }
        })
    }
}

#[derive(Clone, Copy)]
struct TgUploadContext<'a> {
    base: &'a Context,
    payload: &'a Request,
    media: &'a derpi::Media,
}

impl TgUploadContext<'_> {
    async fn upload(self) -> Result<FileMeta> {
        if self.media.mime_type.is_image() {
            self.upload_image().await
        } else {
            self.upload_video().await
        }
    }

    async fn upload_image(self) -> Result<FileMeta> {
        let image_url = &self.media.view_url;

        // FIXME: resize image if it doesn't fit into telegram's limit
        if self.media.aspect_ratio > 20.0 {
            return Err(err_val!(ErrorKind::Todo {
                message: "support for images with aspect ratio > 20"
            }));
        }
        if self.media.height + self.media.width > 10000 {
            return Err(err_val!(ErrorKind::Todo {
                message: "support for images with height + width > 10000"
            }));
        }

        // FIXME: use cached document correctly in inline query

        // Tested that file with this size is the one that can be uploaded.
        // The next file with greater size from derpibooru fails to be uploaded.
        //
        // In fact, there is a 5MB limit for photos uploaded via a direct
        // URL, but it looks like telegram counts megabytes using 1024 as a base,
        // and maybe they allow some margin around the 5 MB boundary, because
        // 1024 * 1024 * 5 == 5242880, but not the value specified here:
        //
        if self.media.size <= 5 * MB {
            // Derpibooru statistics for files according to this limit:
            // - Under: 2_555_839  (95%)
            // - Over:    133_821 (5%)
            self.upload_image_direct().await
        } else if self.media.size <= 10 * MB {
            // Derpibooru statistics for files according to this limit:
            // - Under: 2_651_709 (98.5%)
            // - Over:     37_953 (1.5%)
            self.upload_image_indirect().await
        } else {
            let thumb = self.media.representations.thumb_small.clone();
            let func = |tg_bot: &tg::Bot, chat_id, input_file| {
                tg_bot
                    .send_document(chat_id, input_file)
                    .thumb(InputFile::url(thumb))
            };
            self.upload_imp(func, InputFileKind::DownloadedUrl(image_url.clone()))
                .await
        }
    }

    async fn upload_image_direct(self) -> Result<FileMeta> {
        let input_file = InputFileKind::Url(self.media.view_url.clone());
        match self.upload_imp(tg::Bot::send_photo, input_file).await {
            Ok(file) => return Ok(file),
            Err(err) => {
                warn!(
                    err = tracing_err(&err),
                    derpi_mime = %self.media.mime_type,
                    derpi_size = self.media.size,
                    derpi_id = %self.media.id,
                    "Failed to upload image directly. \
                    Retrying with an intermediate download..."
                );
            }
        }
        self.upload_image_indirect().await
    }

    async fn upload_image_indirect(self) -> Result<FileMeta> {
        self.upload_imp(
            tg::Bot::send_photo,
            InputFileKind::DownloadedUrl(self.media.view_url.clone()),
        )
        .await
    }

    async fn upload_video(self) -> Result<FileMeta> {
        info!("Started converting a video");
        let start = Instant::now();
        let tmp_output = crate::media::convert_to_mp4(&self.media.view_url).await?;
        let actual_size = fs::metadata(&tmp_output).await?.len();
        info!(
            actual_size,
            took = format_args!("{:.2?}", start.elapsed()),
            "Finished converting a video"
        );
        self.upload_imp(
            tg::Bot::send_video,
            InputFileKind::File(tmp_output.to_path_buf()),
        )
        .await
    }

    #[instrument(skip_all, fields(
        tg_method = %S::TYPE,
        derpi_mime = %self.media.mime_type,
        derpi_size = self.media.size,
        derpi_id = %self.media.id,
        tg_input_file = ?input_file,
    ))]
    async fn upload_imp<S>(
        self,
        send_payload_method: impl FnOnce(&tg::Bot, ChatId, InputFile) -> S,
        input_file: InputFileKind,
    ) -> Result<FileMeta>
    where
        S: util::SendPayloadExt,
    {
        info!("Uploading to telegram cache chat");

        let caption = format!(
            "{}\n*Requested by:* {}",
            core_caption(&self.media),
            self.payload.requested_by.md_link()
        );

        let input_file = input_file
            .into_input_file(&self.base.http_client)
            .await?
            .file_name(self.file_name());

        let chat = self.base.cfg.media_cache_chat;
        let msg = send_payload_method(&self.base.bot, chat, input_file)
            .caption(caption)
            .await?;

        find_file(self.media.mime_type, msg)
    }

    fn file_name(&self) -> String {
        fn join_tags(tags: &mut dyn Iterator<Item = &str>) -> String {
            let joined = tags.map(derpi::sanitize_tag).join("+");
            if joined.chars().count() <= 100 {
                return joined;
            }
            joined.chars().take(97).chain(['.', '.', '.']).collect()
        }

        let ratings = join_tags(&mut self.media.rating_tags());
        let artists = join_tags(&mut self.media.artists());

        let prefix = ["derpibooru", ratings.as_str(), artists.as_str()]
            .into_iter()
            .format("-");

        format!(
            "{prefix}-{}.{}",
            self.media.id,
            self.media.mime_type.file_extension()
        )
    }
}

fn find_file(expected: derpi::MimeType, msg: Message) -> Result<FileMeta> {
    use teloxide::types::MediaKind::*;
    let common = assert_matches!(msg.kind, MessageKind::Common(common) => common);

    Ok(match common.media_kind {
        Document(media) => media.document.file,
        Photo(media) => media.photo.into_iter().next().unwrap().file,
        Video(media) => media.video.file,
        media @ (Animation(_) | Audio(_) | Contact(_) | Game(_) | Venue(_) | Location(_)
        | Poll(_) | Sticker(_) | Text(_) | VideoNote(_) | Voice(_) | Migration(_)) => {
            return Err(err_val!(MediaError::UnexpectedMediaKind {
                media,
                expected
            }))
        }
    })
}

pub(crate) fn core_caption(media: &derpi::Media) -> String {
    let artists: Vec<_> = media
        .artists()
        .sorted_unstable()
        .map(|artist| {
            markdown::link(
                derpi::artist_to_webpage_url(artist).as_str(),
                &format!("{}", markdown::escape(artist)),
            )
        })
        .collect();

    let artists = match artists.as_slice() {
        [] => "".to_owned(),
        artists => {
            format!(" by {}", artists.join(", "))
        }
    };

    let ratings = media.rating_tags().join(", ");
    let ratings = if matches!(ratings.as_str(), "" | "safe") {
        "".to_owned()
    } else {
        format!(" \\({}\\)", ratings)
    };

    format!(
        "*Art from {}{artists}{ratings}*",
        markdown::link(
            &String::from(derpi::media_id_to_webpage_url(media.id)),
            r"derpibooru",
        )
    )
}
