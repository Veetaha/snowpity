use super::{Context, Response};
use crate::db::CachedMedia;
use crate::util::http;
use crate::util::prelude::*;
use crate::{derpi, err_val, tg, util, ErrorKind, MediaError, Result};
use assert_matches::assert_matches;
use fs_err::tokio as fs;
use itertools::Itertools;
use metrics_bat::prelude::*;
use reqwest::Url;
use std::fmt;
use std::path::PathBuf;
use std::time::Instant;
use teloxide::prelude::*;
use teloxide::types::{FileMeta, InputFile, MessageKind, User};
use teloxide::utils::markdown;

const KB: u64 = 1024;
const MB: u64 = 1024 * KB;

const KB_F: f64 = KB as f64;
const MB_F: f64 = MB as f64;

metrics_bat::labels! {
    CacheLabels {
        user
    }
    TgUploadLabels {
        user,
        derpi_mime,
        tg_method,
    }
}

metrics_bat::counters! {
    /// Number of times we hit the database cache for derpibooru media
    derpi_cache_hits_total;

    /// Number of times we queried the database cache for derpibooru media
    derpi_cache_queries_total;
}

metrics_bat::histograms! {
    /// Number of seconds it took to upload derpibooru media to Telegram.
    /// It doensn't include the time to query the media from derpibooru and db cache.
    derpi_tg_media_upload_duration_seconds = crate::metrics::DEFAULT_DURATION_BUCKETS;

    /// Number of seconds it took to download media from derpibooru.
    derpi_media_download_duration_seconds = crate::metrics::DEFAULT_DURATION_BUCKETS;

    /// Size of media requested to be uploaded to Telegram
    derpi_tg_media_upload_file_size_bytes = [
        KB_F * 4.,
        KB_F * 16.,
        KB_F * 64.,
        KB_F * 256.,
        MB_F * 1.,
        MB_F * 2.,
        MB_F * 4.,
        MB_F * 6.,
        MB_F * 8.,
        MB_F * 10.,
        MB_F * 20.,
        MB_F * 50.,
    ];
}

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
        ctx.derpi.get_media(payload.media_id),
        ctx.db.media_cache.get_from_derpi(payload.media_id)
    )?;

    let labels = CacheLabels {
        user: payload.requested_by.debug_id(),
    };

    derpi_cache_queries_total(labels.clone()).increment(1);

    if let Some(cached) = cached {
        info!("Returning media from cache");
        derpi_cache_hits_total(labels).increment(1);
        return Ok(Response { media, cached });
    }

    derpi_tg_media_upload_file_size_bytes(labels).record(media.size as f64);

    let cached = TgUploadContext {
        base: &ctx,
        payload: &payload,
        media: &media,
    }
    .upload()
    .await?;

    ctx.db.media_cache.set_derpi(cached.clone()).await?;

    Ok(Response { media, cached })
}

#[derive(strum::AsRefStr)]
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
            Self::DownloadedUrl(url) => InputFile::memory(http_client.get(url).read_bytes().await?),
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
    async fn upload(self) -> Result<CachedMedia> {
        if self.media.mime_type.is_image() {
            self.upload_image().await
        } else {
            self.upload_video().await
        }
    }

    async fn upload_image(self) -> Result<CachedMedia> {
        let max_size = 50 * MB;

        if self.media.size > max_size {
            return Err(err_val!(MediaError::FileTooBig {
                actual: self.media.size,
                max: max_size
            }));
        }

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

        // We shouldn't trust derpibooru with the size of the file.
        // There was a precedent where a media with incorrect size was
        // found. It was reported to derpibooru's Discord:
        // https://discord.com/channels/430829008402251796/438029140659142657/1049534872739389440
        //
        // The media that was reported is https://derpibooru.org/api/v1/json/images/1127198
        // When downloaded, the image's size is 4_941_837 bytes,
        // but the API reports size as 5_259_062.
        //
        // Unfortunately, it does't seem this bug will be fixed anytime soon,
        // so the workaround is falling back to uploading as indirect image
        // or as a document, while optimistically trying the easiest way first

        if self.media.size <= 5 * MB {
            match self.upload_photo_by_url().await {
                Ok(cached) => return Ok(cached),
                Err(err) => {
                    warn!(
                        err = tracing_err(&err),
                        derpi_mime = %self.media.mime_type,
                        derpi_size = self.media.size,
                        derpi_id = %self.media.id,
                        "Failed to upload image using a direct URL. \
                        Falling back to intermediate download of the image..."
                    );
                }
            }
        }

        if self.media.size <= 10 * MB {
            match self.upload_photo_downloaded().await {
                Ok(cached) => return Ok(cached),
                Err(err) => {
                    warn!(
                        err = tracing_err(&err),
                        derpi_mime = %self.media.mime_type,
                        derpi_size = self.media.size,
                        derpi_id = %self.media.id,
                        "Failed to upload image using an intermediate download. \
                        Falling back to direct URL document upload..."
                    );
                }
            }
        }

        if self.media.size <= 20 * MB {
            match self.upload_document_by_url().await {
                Ok(cached) => return Ok(cached),
                Err(err) => {
                    warn!(
                        err = tracing_err(&err),
                        derpi_mime = %self.media.mime_type,
                        derpi_size = self.media.size,
                        derpi_id = %self.media.id,
                        "Failed to upload image using a direct URL document upload. \
                        Falling back to intermediate download of the document..."
                    );
                }
            }
        }

        self.upload_document_downloaded().await
    }

    // Derpibooru statistics for files according to 5 MB limit:
    // - Under: 2_555_839  (95%)
    // - Over:    133_821 (5%)
    async fn upload_photo_by_url(self) -> Result<CachedMedia> {
        let input_file = InputFileKind::Url(self.media.view_url.clone());
        self.upload_imp(tg::Bot::send_photo, input_file).await
    }

    // Derpibooru statistics for files according to 10 MB limit:
    // - Under: 2_651_709 (98.5%)
    // - Over:     37_953 (1.5%)
    async fn upload_photo_downloaded(self) -> Result<CachedMedia> {
        let input_file = InputFileKind::DownloadedUrl(self.media.view_url.clone());
        self.upload_imp(tg::Bot::send_photo, input_file).await
    }

    async fn upload_document_by_url(self) -> Result<CachedMedia> {
        let input_file = InputFileKind::Url(self.media.view_url.clone());
        self.upload_imp(tg::Bot::send_document, input_file).await
    }

    async fn upload_document_downloaded(self) -> Result<CachedMedia> {
        // FIXME: create thumbnails by converting them and coercing to
        // telegram requirements via ffmpeg
        // let thumb = self.media.representations.thumb_tiny.clone();
        // let func = |tg_bot: &tg::Bot, chat_id, input_file| {
        //     tg_bot
        //         .send_document(chat_id, input_file)
        //         .thumb(InputFile::url(thumb))
        // };
        let input_file = InputFileKind::DownloadedUrl(self.media.view_url.clone());
        self.upload_imp(tg::Bot::send_document, input_file).await
    }

    async fn upload_video(self) -> Result<CachedMedia> {
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
        .await?;
        todo!("Video uploads are not implemented yet")
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
    ) -> Result<CachedMedia>
    where
        S: util::SendPayloadExt,
    {
        info!("Uploading to telegram cache chat");

        let derpi_mime: &'static str = self.media.mime_type.into();
        let tg_method: &'static str = S::TYPE.into();

        let labels = TgUploadLabels {
            user: self.payload.requested_by.debug_id(),
            derpi_mime,
            tg_method,
        };

        let caption = format!(
            "{}\n*Requested by: {}\\. Uploaded as {}*",
            core_caption(&self.media),
            self.payload.requested_by.md_link(),
            S::TYPE,
        );

        let measure_download = matches!(input_file, InputFileKind::DownloadedUrl(_));
        let file_fut = input_file.into_input_file(&self.base.http_client);

        let file = if measure_download {
            file_fut
                .record_duration(derpi_media_download_duration_seconds, labels.clone())
                .await
        } else {
            file_fut.await
        }?
        .file_name(file_name(&self.media));

        let chat = self.base.cfg.media_cache_chat;
        let msg = send_payload_method(&self.base.bot, chat, file)
            .caption(caption)
            .into_future()
            .record_duration(derpi_tg_media_upload_duration_seconds, labels)
            .await?;

        let file_meta = find_file(self.media.mime_type, msg)?;

        Ok(CachedMedia {
            derpi_id: self.media.id,
            tg_file_id: file_meta.id,
            tg_file_type: S::TYPE,
        })
    }
}

/// Short name of the file (not more than 255 characters) for the media
pub(crate) fn file_name(media: &derpi::Media) -> String {
    fn join_tags(tags: &mut dyn Iterator<Item = &str>) -> String {
        let joined = tags.map(derpi::sanitize_tag).join("+");
        if joined.chars().count() <= 100 {
            return joined;
        }
        joined.chars().take(97).chain(['.', '.', '.']).collect()
    }

    let ratings = join_tags(&mut media.rating_tags());
    let artists = join_tags(&mut media.artists());

    let prefix = ["derpibooru", ratings.as_str(), artists.as_str()]
        .into_iter()
        .format("-");

    format!("{prefix}-{}.{}", media.id, media.mime_type.file_extension())
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
