use super::{Context, Response, TgFileType};
use crate::db::CachedMedia;
use crate::observability::logging::prelude::*;
use crate::prelude::*;
use crate::tg::inline_query::media_cache::{Artist, MediaHostingSpecific, MediaMeta};
use crate::{derpi, err_val, MediaError, Result};
use assert_matches::assert_matches;
use futures::prelude::*;
use itertools::Itertools;
use metrics_bat::prelude::*;
use reqwest::Url;
use teloxide::prelude::*;
use teloxide::types::{FileMeta, InputFile, MessageKind, User};

const KB: u64 = 1024;
const MB: u64 = 1024 * KB;

const KB_F: f64 = KB as f64;
const MB_F: f64 = MB as f64;

metrics_bat::labels! {
    DownloadLabels {
        derpi_mime,
        tg_file_type,
    }
    TgUploadLabels {
        derpi_mime,
        tg_file_type,
        tg_upload_method,
    }
}

#[derive(strum::IntoStaticStr, Clone)]
enum TgUploadMethod {
    DirectUrl(Url),
    Downloaded(Downloaded),
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
        ctx.derpi
            .get_media(payload.media_id)
            .instrument(info_span!("Fetching media meta from Derpibooru")),
        ctx.db
            .tg_media_cache
            .get_from_derpi(payload.media_id)
            .with_duration_log("Reading the cache from the database"),
    )?;

    derpi_cache_queries_total(vec![]).increment(1);

    if let Some(cached) = cached {
        info!("Returning media from cache");
        derpi_cache_hits_total(vec![]).increment(1);

        return Ok(make_response(media, cached));
    }

    derpi_tg_media_upload_file_size_bytes(vec![]).record(media.size as f64);

    let cached = TgUploadContext {
        base: &ctx,
        payload: &payload,
        media: &media,
    }
    .upload()
    .await?;

    ctx.db.tg_media_cache.set_derpi(cached.clone()).await?;

    Ok(make_response(media, cached))
}

fn make_response(media: derpi::Media, cached: CachedMedia) -> Response {
    Response {
        tg_file_id: cached.tg_file_id,
        tg_file_type: cached.tg_file_type,
        meta: media.into(),
    }
}

impl From<derpi::Media> for MediaMeta {
    fn from(media: derpi::Media) -> Self {
        Self {
            artists: media
                .artists()
                .map(|artist| Artist {
                    link: derpi::artist_to_webpage_url(artist),
                    name: artist.to_owned(),
                })
                .collect(),
            link: media.id.to_webpage_url(),
            hosting_specific: MediaHostingSpecific::Derpibooru {
                ratings: media.rating_tags().map(ToOwned::to_owned).collect(),
            },
        }
    }
}

#[derive(Clone, Copy)]
struct TgUploadContext<'a> {
    base: &'a Context,
    payload: &'a Request,
    media: &'a derpi::Media,
}

macro_rules! try_return_upload {
    ($method_ctx:expr) => {
        if let Ok(cached) = $method_ctx.upload_warn_on_error().await {
            return Ok(cached);
        }
    };
}

impl TgUploadContext<'_> {
    async fn upload(&self) -> Result<CachedMedia> {
        use derpi::MimeType::*;

        match self.media.mime_type {
            ImageJpeg | ImagePng | ImageSvgXml => self.upload_image().await,
            ImageGif => self.upload_mpeg4_gif().await,
            VideoWebm => self.upload_video().await,
        }
    }
    async fn upload_image(&self) -> Result<CachedMedia> {
        // FIXME: resize the image if it doesn't fit into telegram's limit
        if self.media.aspect_ratio > 20.0 || self.media.height + self.media.width > 10000 {
            return self
                .upload_document(MaybeDownloaded::None(self.media.view_url.clone()))
                .await;
        }

        let url = &self.media.view_url;
        let ctx = self.file_kind(TgFileType::Photo);

        if self.media.size <= 5 * MB {
            try_return_upload!(ctx.direct_url(&url));
        }

        let max_size = 10 * MB;

        let maybe_downloaded = if self.media.size > max_size {
            MaybeDownloaded::None(url.clone())
        } else {
            let downloaded = ctx.download_media(&url).await?;
            if downloaded.size < max_size {
                try_return_upload!(ctx.downloaded(&downloaded));
            }
            MaybeDownloaded::Some(downloaded)
        };

        self.upload_document(maybe_downloaded).await
    }

    async fn upload_mpeg4_gif(&self) -> Result<CachedMedia> {
        self.upload_mp4(TgFileType::Mpeg4Gif).await
    }

    async fn upload_video(&self) -> Result<CachedMedia> {
        self.upload_mp4(TgFileType::Video).await
    }

    async fn upload_mp4(&self, file_kind: TgFileType) -> Result<CachedMedia> {
        let url = self.media.unwrap_mp4_url();

        let ctx = self.file_kind(file_kind);

        // We can't rely on the size of the media, because it's not the size of MP4
        // do this optimization with direct URL upload won't always work
        if self.media.size <= 20 * MB {
            try_return_upload!(ctx.direct_url(&url));
        }

        let downloaded = ctx.download_media(&url).await?;

        downloaded.try_size_less_than(50 * MB)?;

        ctx.downloaded(&downloaded).upload().await
    }

    async fn upload_document(&self, maybe_downloaded: MaybeDownloaded) -> Result<CachedMedia> {
        let ctx = self.file_kind(TgFileType::Document);

        if self.media.size <= 20 * MB {
            if let MaybeDownloaded::None(url) = &maybe_downloaded {
                try_return_upload!(ctx.direct_url(url));
            }
        }

        let downloaded = match maybe_downloaded {
            MaybeDownloaded::None(url) => ctx.download_media(&url).await?,
            MaybeDownloaded::Some(downloaded) => downloaded,
        };

        downloaded.try_size_less_than(50 * MB)?;

        ctx.downloaded(&downloaded).upload().await
    }

    fn file_kind(&self, tg_file_type: TgFileType) -> TgUploadKindContext<'_> {
        TgUploadKindContext {
            base: self,
            tg_file_type,
        }
    }
}

enum MaybeDownloaded {
    Some(Downloaded),
    None(Url),
}

#[derive(Clone)]
struct Downloaded {
    file: InputFile,
    size: u64,
    url: Url,
}

impl Downloaded {
    /// We shouldn't trust derpibooru with the size of the file.
    /// There was a precedent where a media with incorrect size was
    /// found. It was reported to derpibooru's Discord:
    /// https://discord.com/channels/430829008402251796/438029140659142657/1049534872739389440
    ///
    /// The media that was reported is https://derpibooru.org/api/v1/json/images/1127198
    /// When downloaded, the image's size is 4_941_837 bytes,
    /// but the API reports size as 5_259_062.
    ///
    /// Unfortunately, it does't seem this bug will be fixed anytime soon,
    /// so the workaround is falling back to uploading as indirect image
    /// or as a document, while optimistically trying the easiest way first
    fn try_size_less_than(&self, max_size: u64) -> Result {
        if self.size <= max_size {
            return Ok(());
        }
        Err(err_val!(MediaError::FileTooBig {
            actual: self.size as u64,
            max: max_size
        }))
    }
}

#[derive(Clone, Copy)]
struct TgUploadKindContext<'a> {
    base: &'a TgUploadContext<'a>,
    tg_file_type: TgFileType,
}

impl TgUploadKindContext<'_> {
    async fn download_media(&self, download_url: &Url) -> Result<Downloaded> {
        let labels = DownloadLabels {
            derpi_mime: <&'static str>::from(self.base.media.mime_type),
            tg_file_type: <&'static str>::from(self.tg_file_type),
        };

        let bytes = self
            .download_media_imp(download_url)
            .record_duration(derpi_media_download_duration_seconds, labels)
            .await?;

        Ok(Downloaded {
            size: bytes.len() as u64,
            file: InputFile::memory(bytes),
            url: download_url.clone(),
        })
    }

    async fn download_media_imp(&self, download_url: &Url) -> Result<bytes::Bytes> {
        let (bytes, duration) = self
            .base
            .base
            .http_client
            .get(download_url.clone())
            .read_bytes()
            .with_duration_ok()
            .await?;

        info!(
            actual_size = bytes.len(),
            duration = tracing_duration(duration),
            "Downloaded file"
        );

        return Ok(bytes);
    }

    fn method(&self, tg_upload_method: TgUploadMethod) -> TgUploadMethodContext<'_> {
        TgUploadMethodContext {
            base: self,
            tg_upload_method,
        }
    }

    fn direct_url(&self, url: &Url) -> TgUploadMethodContext<'_> {
        self.method(TgUploadMethod::DirectUrl(url.clone()))
    }

    fn downloaded(&self, downloaded: &Downloaded) -> TgUploadMethodContext<'_> {
        self.method(TgUploadMethod::Downloaded(downloaded.clone()))
    }
}

struct TgUploadMethodContext<'a> {
    base: &'a TgUploadKindContext<'a>,
    tg_upload_method: TgUploadMethod,
}

impl TgUploadMethodContext<'_> {
    fn span_for_upload(&self) -> tracing::Span {
        let download_url = match &self.tg_upload_method {
            TgUploadMethod::DirectUrl(url) => url,
            TgUploadMethod::Downloaded(downloaded) => &downloaded.url,
        };
        info_span!(
            "tg_upload",
            tg_file_type = %self.base.tg_file_type,
            tg_upload_method = %<&'static str>::from(&self.tg_upload_method),
            download_url = %download_url,
            derpi_mime = %self.base.base.media.mime_type,
            derpi_size = self.base.base.media.size,
            derpi_id = %self.base.base.media.id,
        )
    }

    fn warn_failed_upload(&self, err: &crate::Error) {
        warn!(err = tracing_err(err), "Failed to upload media to telegram");
    }

    #[instrument(skip_all)]
    async fn upload_warn_on_error(&self) -> Result<CachedMedia> {
        self.upload_imp()
            .inspect_err(|err| self.warn_failed_upload(err))
            .instrument(self.span_for_upload())
            .await
    }

    #[instrument(skip_all)]
    async fn upload(&self) -> Result<CachedMedia> {
        self.upload_imp().instrument(self.span_for_upload()).await
    }

    async fn upload_imp(&self) -> Result<CachedMedia> {
        let (url, input_file) = match &self.tg_upload_method {
            TgUploadMethod::DirectUrl(url) => (url, InputFile::url(url.clone())),
            TgUploadMethod::Downloaded(downloaded) => (&downloaded.url, downloaded.file.clone()),
        };

        let input_file = input_file.file_name(file_name(url, self.base.base.media));

        self.into_cached_media(self.upload_file(input_file).await?)
    }

    async fn upload_file(&self, input_file: InputFile) -> Result<Message, teloxide::RequestError> {
        self.base
            .tg_file_type
            .upload(
                &self.base.base.base.bot,
                self.base.base.base.cfg.media_cache_chat,
                input_file,
                self.caption(),
            )
            .with_duration_log("Send file to telegram")
            .record_duration(derpi_tg_media_upload_duration_seconds, self.upload_labels())
            .await
    }

    fn upload_labels(&self) -> TgUploadLabels<&'static str, &'static str, &'static str> {
        TgUploadLabels {
            derpi_mime: <&'static str>::from(self.base.base.media.mime_type),
            tg_file_type: <&'static str>::from(&self.tg_upload_method),
            tg_upload_method: <&'static str>::from(self.base.tg_file_type),
        }
    }

    fn caption(&self) -> String {
        let core_caption = MediaMeta::from(self.base.base.media.clone()).caption();
        let requested_by = self.base.base.payload.requested_by.md_link();
        let via_method = match &self.tg_upload_method {
            TgUploadMethod::DirectUrl(_) => "direct URL",
            TgUploadMethod::Downloaded(_) => "downloaded",
        };
        let file_kind = self.base.tg_file_type.to_string().to_lowercase();
        format!(
            "{core_caption}\n*Requested by: {requested_by}\\\n\
            Uploaded as {file_kind} {via_method}*",
        )
    }

    fn into_cached_media(&self, msg: Message) -> Result<CachedMedia> {
        let (actual_file_type, file_meta) = find_file(self.base.base.media.mime_type, msg)?;

        if actual_file_type != self.base.tg_file_type {
            info!(
                %actual_file_type,
                requested_file_type = %self.base.tg_file_type,
                "Actual uploaded tg file type differs from requested",
            )
        }

        Ok(CachedMedia {
            derpi_id: self.base.base.media.id,
            tg_file_id: file_meta.id,
            tg_file_type: actual_file_type,
        })
    }
}

/// Short name of the file (not more than 255 characters) for the media
pub(crate) fn file_name(url: &Url, media: &derpi::Media) -> String {
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
        .filter(|s| !s.is_empty())
        .format("-");

    // FIXME: this file type detection influences how telegram proceses the file.
    // For example, if we send_video with wrong extension, then it will be registered
    // as a document instead of video, even though it will be returned as a video kind in `Message`
    let file_extension = url
        .path()
        .rsplit('.')
        .next()
        .unwrap_or_else(|| media.mime_type.file_extension());

    format!("{prefix}-{}.{}", media.id, file_extension)
}

fn find_file(expected: derpi::MimeType, msg: Message) -> Result<(TgFileType, FileMeta)> {
    use teloxide::types::MediaKind::*;
    let common = assert_matches!(msg.kind, MessageKind::Common(common) => common);

    Ok(match common.media_kind {
        Document(media) => (TgFileType::Document, media.document.file),
        Photo(media) => (
            TgFileType::Photo,
            media.photo.into_iter().next().unwrap().file,
        ),
        Video(media) => (TgFileType::Video, media.video.file),
        Animation(media) => (TgFileType::Mpeg4Gif, media.animation.file),
        media @ (Audio(_) | Contact(_) | Game(_) | Venue(_) | Location(_) | Poll(_)
        | Sticker(_) | Text(_) | VideoNote(_) | Voice(_) | Migration(_)) => {
            return Err(err_val!(MediaError::UnexpectedMediaKind {
                media,
                expected
            }))
        }
    })
}
