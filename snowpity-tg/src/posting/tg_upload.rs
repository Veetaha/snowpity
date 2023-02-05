use crate::observability::logging::prelude::*;
use crate::posting::platform::prelude::*;
use crate::posting::{Context, PostingError};
use crate::prelude::*;
use crate::util::DynError;
use crate::{err, Result};
use assert_matches::assert_matches;
use derive_more::Deref;
use from_variants::FromVariants;
use futures::prelude::*;
use metrics_bat::prelude::*;
use teloxide::prelude::*;
use teloxide::types::{FileMeta, InputFile, MessageKind};

const KB_F: f64 = KB as f64;
const MB_F: f64 = MB as f64;

metrics_bat::labels! {
    DownloadLabels {
        blob_kind,
        tg_blob_kind,
    }
    TgUploadLabels {
        blob_kind,
        tg_blob_kind,
        tg_upload_method,
    }
}

#[derive(strum::IntoStaticStr)]
enum TgUploadMethod<'a> {
    DirectUrl,
    Downloaded(Downloaded<&'a LocalBlob>),
}

const BLOB_SIZE_BUCKETS: &[f64] = &[
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

metrics_bat::histograms! {
    /// Number of seconds it took to upload a blob to Telegram.
    /// It doensn't include the time to query the blob from the posting platform
    /// and db cache.
    blob_tg_upload_duration_seconds = crate::metrics::DEFAULT_DURATION_BUCKETS;

    /// Number of seconds it took to download blob from the posting platform.
    blob_download_duration_seconds = crate::metrics::DEFAULT_DURATION_BUCKETS;

    /// Size of the blob originally downloaded from the posting platform.
    blob_original_size_bytes = BLOB_SIZE_BUCKETS;

    /// Size of blob to be uploaded to Telegram after intermediate processing (if any)
    blob_uploaded_size_bytes = BLOB_SIZE_BUCKETS;
}

pub(crate) async fn upload(
    base: &Context,
    post: &BasePost,
    blob: &Blob,
    requested_by: &teloxide::types::User,
) -> Result<TgFileMeta> {
    TgUploadContext {
        base,
        post,
        blob,
        requested_by,
    }
    .upload()
    .await
}

#[derive(Clone, Copy, Deref)]
struct TgUploadContext<'a> {
    #[deref(forward)]
    base: &'a Context,
    post: &'a BasePost,
    blob: &'a Blob,
    requested_by: &'a teloxide::types::User,
}

macro_rules! try_return_upload {
    ($method_ctx:expr) => {
        if let Ok(cached) = $method_ctx.upload_warn_on_error().await {
            return Ok(cached);
        }
    };
}

impl TgUploadContext<'_> {
    async fn upload(&self) -> Result<TgFileMeta> {
        if let BlobSize::Approx(size) = self.blob.size {
            blob_size_bytes(vec![]).record(size as f64);
        }

        use crate::posting::BlobKind::*;

        match self.blob.kind {
            ImageJpeg | ImagePng | ImageSvg => self.upload_image().await,
            AnimationMp4 => self.upload_mpeg4_gif().await,
            VideoMp4 => self.upload_video().await,
            AnimationGif => self.upload_gif_as_mpeg4_gif().await,
        }
    }

    async fn upload_image(&self) -> Result<TgFileMeta> {
        let dim = &self.blob.dimensions;

        // FIXME: resize the image if it doesn't fit into telegram's limit
        if dim.aspect_ratio() > 20.0 || dim.height + dim.width > 10000 {
            return self.upload_document(MaybeDownloaded::None).await;
        }

        let ctx = self.file_kind(TgFileKind::Photo);
        let approx_max_size = self.blob.size.approx_max();

        if approx_max_size <= MAX_PHOTO_BY_URL_SIZE {
            try_return_upload!(ctx.direct_url());
        }

        let maybe_downloaded = if approx_max_size > MAX_PHOTO_BY_DIRECT_SIZE {
            MaybeDownloaded::None
        } else {
            let downloaded = ctx.download_blob_to_ram().await?;
            if downloaded.size < MAX_PHOTO_BY_DIRECT_SIZE {
                try_return_upload!(ctx.downloaded(&downloaded));
            }
            MaybeDownloaded::Some(downloaded)
        };

        self.upload_document(maybe_downloaded).await
    }

    fn upload_gif_as_mpeg4_gif(&self) -> Result<TgFileMeta> {
        let ctx = self.file_kind(TgFileKind::Mpeg4Gif);
    }

    async fn upload_mpeg4_gif(&self) -> Result<TgFileMeta> {
        self.upload_mp4(TgFileKind::Mpeg4Gif).await
    }

    async fn upload_video(&self) -> Result<TgFileMeta> {
        self.upload_mp4(TgFileKind::Video).await
    }

    async fn upload_mp4(&self, file_kind: TgFileKind) -> Result<TgFileMeta> {
        let ctx = self.file_kind(file_kind);

        // We can't rely on the size of the blob, because it's not the size of MP4
        // do this optimization with direct URL upload won't always work
        if self.blob.size.approx_max() <= MAX_FILE_BY_URL_SIZE {
            try_return_upload!(ctx.direct_url());
        }

        let downloaded = ctx
            .download_blob_to_ram(MAX_FILE_BY_DIRECT_SIZE)
            .await?
            .upcast();

        downloaded.try_size_less_than(MAX_FILE_BY_DIRECT_SIZE)?;

        ctx.downloaded(downloaded).upload().await
    }

    async fn upload_document(&self, maybe_downloaded: MaybeDownloaded) -> Result<TgFileMeta> {
        let ctx = self.file_kind(TgFileKind::Document);

        if self.blob.size.approx_max() <= MAX_FILE_BY_URL_SIZE {
            if let MaybeDownloaded::None = &maybe_downloaded {
                try_return_upload!(ctx.direct_url());
            }
        }

        let downloaded = match maybe_downloaded {
            MaybeDownloaded::None => ctx.download_blob_to_ram().await?,
            MaybeDownloaded::Some(downloaded) => downloaded,
        };

        downloaded.try_size_less_than(MAX_FILE_BY_DIRECT_SIZE)?;

        ctx.downloaded(&downloaded).upload().await
    }

    fn file_kind(&self, tg_file_type: TgFileKind) -> TgUploadKindContext<'_> {
        TgUploadKindContext {
            base: self,
            tg_file_type,
        }
    }
}

enum MaybeDownloaded {
    Some(Downloaded<LocalBlob>),
    None,
}

// #[derive(Clone)]
struct Downloaded<B> {
    blob: B,
    size: u64,
}

#[derive(FromVariants)]
enum LocalBlob {
    Ram(bytes::Bytes),
    Disk(tempfile::TempPath),
}

impl<B> Downloaded<B> {
    fn upcast(self) -> Downloaded<LocalBlob>
    where
        B: Into<LocalBlob>,
    {
        Downloaded {
            blob: self.blob.into(),
            size: self.size,
        }
    }
}

#[derive(Clone, Copy, Deref)]
struct TgUploadKindContext<'a> {
    #[deref(forward)]
    base: &'a TgUploadContext<'a>,
    tg_file_type: TgFileKind,
}

impl TgUploadKindContext<'_> {
    async fn download_blob_to_ram(&self, max_size: u64) -> Result<Downloaded<bytes::Bytes>> {
        self.download_blob_imp(reqwest::Response::bytes, max_size)
            .await
    }

    async fn download_blob_to_disk(&self, max_size: u64) -> Result<Downloaded<tempfile::TempPath>> {
        self.download_blob_imp(reqwest::Response::read_to_temp_file, max_size)
            .await
    }

    async fn download_blob_imp<Payload, E, Fut>(
        &self,
        download: fn(reqwest::Response) -> Fut,
        max_size: u64,
    ) -> Result<Downloaded<Payload>>
    where
        Fut: Future<Output = Result<Payload, E>> + Send,
        E: Into<Box<DynError>>,
    {
        let labels = DownloadLabels {
            blob_kind: <&'static str>::from(self.blob.kind),
            tg_blob_kind: <&'static str>::from(self.tg_file_type),
        };

        let (downloaded, duration) = async {
            let (response, content_length) = self
                .http
                .get(self.blob.download_url.clone())
                .try_send_with_content_length()
                .await?;

            // We shouldn't trust posting platforms with the size of the file.
            // There was a precedent where a blob with incorrect size was
            // found in derpibooru. It was reported to derpibooru's Discord:
            // https://discord.com/channels/430829008402251796/438029140659142657/1049534872739389440
            //
            // The blob that was reported is https://derpibooru.org/api/v1/json/images/1127198
            // When downloaded, the image's size is 4_941_837 bytes,
            // but the API reports size as 5_259_062.
            //
            // Unfortunately, it does't seem this bug will be fixed anytime soon,
            // so the workaround is using the content legth information from the
            // blob URL download endpoint
            if content_length > max_size {
                return Err(err!(PostingError::BlobTooBig {
                    actual: content_length,
                    max: max_size
                }));
            }

            let payload = download(response).await.fatal_ctx(|| {
                format!(
                    "Failed to download blob from URL `{}`",
                    self.blob.download_url
                )
            })?;

            Ok::<_, crate::Error>(Downloaded {
                blob: payload,
                size: content_length,
            })
        }
        .record_duration(blob_download_duration_seconds, labels)
        .with_duration_ok()
        .await?;

        blob_original_size_bytes(vec![]).record(downloaded.size as f64);

        info!(
            actual_size = downloaded.size,
            duration = tracing_duration(duration),
            "Downloaded file"
        );

        Ok(downloaded)
    }

    fn method(&self, tg_upload_method: TgUploadMethod) -> TgUploadMethodContext<'_> {
        TgUploadMethodContext {
            base: self,
            tg_upload_method,
        }
    }

    fn direct_url(&self) -> TgUploadMethodContext<'_> {
        self.method(TgUploadMethod::DirectUrl)
    }

    fn downloaded(&self, downloaded: Downloaded<&LocalBlob>) -> TgUploadMethodContext<'_> {
        self.method(TgUploadMethod::Downloaded(downloaded))
    }
}

#[derive(Deref)]
struct TgUploadMethodContext<'a> {
    #[deref(forward)]
    base: &'a TgUploadKindContext<'a>,
    tg_upload_method: TgUploadMethod<'a>,
}

impl TgUploadMethodContext<'_> {
    fn span_for_upload(&self) -> tracing::Span {
        info_span!(
            "tg_upload",
            tg_file_type = %self.tg_file_type,
            tg_upload_method = %<&'static str>::from(&self.tg_upload_method),
            download_url = %self.blob.download_url,
            blob_kind = %self.blob.kind,
            blob_size = ?self.blob.size,
            blob_id = ?self.blob.id,
            post_id = ?self.post.id,
        )
    }

    fn warn_failed_upload(&self, err: &crate::Error) {
        warn!(err = tracing_err(err), "Failed to upload blob to telegram");
    }

    #[instrument(skip_all)]
    async fn upload_warn_on_error(&self) -> Result<TgFileMeta> {
        self.upload_imp()
            .inspect_err(|err| self.warn_failed_upload(err))
            .instrument(self.span_for_upload())
            .await
    }

    #[instrument(skip_all)]
    async fn upload(&self) -> Result<TgFileMeta> {
        self.upload_imp().instrument(self.span_for_upload()).await
    }

    async fn upload_imp(&self) -> Result<TgFileMeta> {
        let input_file = match &self.tg_upload_method {
            TgUploadMethod::DirectUrl => InputFile::url(self.blob.download_url.clone()),
            TgUploadMethod::Downloaded(downloaded) => downloaded.file.clone(),
        };

        let input_file = input_file.file_name(self.blob.tg_file_name(self.post));

        self.make_cached_blob(self.upload_file(input_file).await?)
    }

    async fn upload_file(&self, input_file: InputFile) -> Result<Message, teloxide::RequestError> {
        self.base
            .tg_file_type
            .upload(
                &self.bot,
                self.config.blob_cache_chat,
                input_file,
                self.caption(),
            )
            .with_duration_log("Send file to telegram")
            .record_duration(blob_tg_upload_duration_seconds, self.upload_labels())
            .await
    }

    fn upload_labels(&self) -> TgUploadLabels<&'static str, &'static str, &'static str> {
        TgUploadLabels {
            blob_kind: <&'static str>::from(self.blob.kind),
            tg_blob_kind: <&'static str>::from(&self.tg_upload_method),
            tg_upload_method: <&'static str>::from(self.tg_file_type),
        }
    }

    fn caption(&self) -> String {
        let core_caption = self.post.caption();
        let requested_by = self.requested_by.md_link();
        let via_method = match &self.tg_upload_method {
            TgUploadMethod::DirectUrl => "direct URL",
            TgUploadMethod::Downloaded(_) => "downloaded",
        };
        let file_kind = self.base.tg_file_type.to_string().to_lowercase();
        format!(
            "{core_caption}\n*Requested by: {requested_by}\\\n\
            Uploaded as {file_kind} {via_method}*",
        )
    }

    fn make_cached_blob(&self, msg: Message) -> Result<TgFileMeta> {
        let (actual_file_kind, file_meta) = self.find_file(msg)?;

        if actual_file_kind != self.tg_file_type {
            info!(
                %actual_file_kind,
                requested_file_type = %self.base.tg_file_type,
                "Actual uploaded tg file type differs from requested",
            )
        }

        Ok(TgFileMeta {
            id: file_meta.id,
            kind: actual_file_kind,
        })
    }

    fn find_file(&self, msg: Message) -> Result<(TgFileKind, FileMeta)> {
        use teloxide::types::MediaKind::*;
        let common = assert_matches!(msg.kind, MessageKind::Common(common) => common);

        Ok(match common.media_kind {
            Document(blob) => (TgFileKind::Document, blob.document.file),
            Photo(blob) => (
                TgFileKind::Photo,
                blob.photo.into_iter().next().unwrap().file,
            ),
            Video(blob) => (TgFileKind::Video, blob.video.file),
            Animation(blob) => (TgFileKind::Mpeg4Gif, blob.animation.file),
            actual => {
                return Err(err!(PostingError::UnexpectedMediaKind {
                    actual,
                    expected: self.blob.kind,
                }))
            }
        })
    }
}
