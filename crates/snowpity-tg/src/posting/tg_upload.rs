use crate::observability::logging::prelude::*;
use crate::posting::platform::prelude::*;
use crate::posting::{PostingContext, PostingError};
use crate::prelude::*;
use crate::util::units::MB;
use crate::util::{display, media_conv, DynError};
use crate::{err, fatal, Result};
use assert_matches::assert_matches;
use derive_more::Deref;
use from_variants::FromVariants;
use fs_err::tokio as fs;
use futures::prelude::*;
use metrics_bat::prelude::*;
use teloxide::prelude::*;
use teloxide::types::{FileMeta, InputFile, MessageKind};

/// If the blob is larger than this, then we will refuse to download it, because
/// it's too big for us to handle, or it could be a malicious blob.
const MAX_DOWNLOAD_SIZE: u64 = 200 * MB;

/// The images must fit into a box with the side of this size.
/// If they don't, then Telegram resizes them to fit.
///
/// This value was inferred from experiments. Telegram Desktop
/// and IOS apps use this value, but Android app displays images
/// with the side of 1280 at the time of this writing, even when
/// a higher resolution is available.
const MAX_LOSSLESS_TG_IMAGE_RESOLUTION: u32 = 2560;

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
    Url,
    Multipart(LocalBlob<&'a LocalBlobKind>),
}

metrics_bat::histograms! {
    /// Number of seconds it took to upload a blob to Telegram.
    /// It doensn't include the time to query the blob from the posting platform
    /// and db cache.
    blob_tg_upload_duration_seconds = crate::metrics::DEFAULT_DURATION_BUCKETS;

    /// Number of seconds it took to download blob from the posting platform.
    blob_download_duration_seconds = crate::metrics::DEFAULT_DURATION_BUCKETS;

    /// Size of the blob originally downloaded from the posting platform.
    blob_original_size_bytes = crate::metrics::DEFAULT_BLOB_SIZE_BUCKETS;

    /// Size of blob to be uploaded to Telegram after intermediate processing (if any)
    blob_uploaded_size_bytes = crate::metrics::DEFAULT_BLOB_SIZE_BUCKETS;
}

pub(crate) async fn upload(
    base: &PostingContext,
    post: &BasePost,
    blob: MultiBlob,
    requested_by: &teloxide::types::User,
) -> Result<CachedBlob> {
    let mut last_error = None;

    for repr in blob.repr {
        let blob = UniBlob {
            id: blob.id.clone(),
            repr,
        };
        let ctx = TgUploadContext {
            base,
            post,
            blob,
            requested_by,
        };

        let err = match ctx.upload().await {
            Ok(tg_file) => {
                return Ok(CachedBlob {
                    blob: ctx.blob,
                    tg_file,
                })
            }
            Err(err) => err,
        };

        warn!(err = tracing_err(&err), "Failed to upload blob to Telegram");

        last_error = Some(err);
    }

    Err(last_error.unwrap_or_else(|| fatal!("The list of representations is empty")))
}

#[derive(Clone, Deref)]
struct TgUploadContext<'a> {
    #[deref(forward)]
    base: &'a PostingContext,
    post: &'a BasePost,
    blob: UniBlob,
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
        use BlobKind::*;
        match self.blob.repr.kind {
            ImageJpeg | ImagePng | ImageSvg => self.upload_image().await,
            AnimationMp4 => self.upload_mpeg4_gif().await,
            VideoMp4 => self.upload_video().await,
            AnimationGif => self.upload_gif_as_mpeg4_gif().await,
        }
    }

    async fn upload_image(&self) -> Result<TgFileMeta> {
        let dim = &self.blob.repr.dimensions;

        if let Some(dim) = dim {
            let span = info_span!("image_dimensions", height = dim.height, width = dim.width);

            if dim.aspect_ratio() > 20.0 {
                return self
                    .upload_document(MaybeLocalBlob::None)
                    .instrument(span)
                    .await;
            }

            if dim.height + dim.width > 10000 {
                return self.resize_and_upload_image().instrument(span).await;
            }
        }

        let ctx = self.file_kind(TgFileKind::Photo);

        let max_size = self.blob.repr.size.to_max_or_zero();

        if max_size <= MAX_TG_PHOTO_SIZE.by_url {
            try_return_upload!(ctx.by_url());
        }

        if max_size > MAX_TG_PHOTO_SIZE.by_multipart {
            return self.upload_document(MaybeLocalBlob::None).await;
        }

        let local_blob = ctx
            .download_blob_to_ram(MAX_TG_FILE_SIZE.by_multipart)
            .await?
            .upcast();

        self.upload_local_image(local_blob).await
    }

    async fn upload_local_image(&self, image: LocalBlob<LocalBlobKind>) -> Result<TgFileMeta> {
        let ctx = self.file_kind(TgFileKind::Photo);

        // FIXME: try resizing the image to a smaller if it's too big
        if image.size < MAX_TG_PHOTO_SIZE.by_multipart {
            try_return_upload!(ctx.by_multipart(&image));
        }

        self.upload_document(MaybeLocalBlob::Some(image)).await
    }

    async fn resize_and_upload_image(&self) -> Result<TgFileMeta> {
        let ctx = self.file_kind(TgFileKind::Photo);

        let image = ctx.download_blob_to_ram(MAX_DOWNLOAD_SIZE).await?;

        // FIXME: send too large images as documents (check width * height)
        // We don't want to allocate a lot of memory for resizing
        let result = media_conv::resize_image_to_bounding_box(
            image.blob.clone(),
            MAX_LOSSLESS_TG_IMAGE_RESOLUTION,
        )
        .await;

        let image = match result {
            Ok(image) => image,
            Err(err) => {
                warn!(err = tracing_err(&err), "Failed to resize image");
                return ctx
                    .upload_document(MaybeLocalBlob::Some(image.upcast()))
                    .await;
            }
        };

        let image = LocalBlob {
            size: u64::try_from(image.len()).unwrap(),
            blob: image,
        }
        .upcast();

        self.upload_local_image(image).await
    }

    async fn upload_gif_as_mpeg4_gif(&self) -> Result<TgFileMeta> {
        let ctx = self.file_kind(TgFileKind::Mpeg4Gif);

        let local_blob = ctx.download_blob_to_disk(MAX_DOWNLOAD_SIZE).await?;

        let output = media_conv::gif_to_mp4(&local_blob.blob).await?;

        let size = fs::metadata(&output)
            .await
            .fatal_ctx(|| "Failed to read generated GIF meta")?
            .len();

        let local_blob = LocalBlob { blob: output, size };

        ctx.by_multipart(&local_blob.upcast()).upload().await
    }

    async fn upload_mpeg4_gif(&self) -> Result<TgFileMeta> {
        self.upload_mp4(TgFileKind::Mpeg4Gif).await
    }

    async fn upload_video(&self) -> Result<TgFileMeta> {
        self.upload_mp4(TgFileKind::Video).await
    }

    async fn upload_mp4(&self, file_kind: TgFileKind) -> Result<TgFileMeta> {
        let ctx = self.file_kind(file_kind);

        if self.blob.repr.size.to_max_or_zero() <= MAX_TG_FILE_SIZE.by_url {
            try_return_upload!(ctx.by_url());
        }

        let local_blob = ctx
            .download_blob_to_ram(MAX_TG_FILE_SIZE.by_multipart)
            .await?
            .upcast();

        ctx.by_multipart(&local_blob).upload().await
    }

    async fn upload_document(&self, maybe_local_blob: MaybeLocalBlob) -> Result<TgFileMeta> {
        let ctx = self.file_kind(TgFileKind::Document);

        if self.blob.repr.size.to_max_or_zero() <= MAX_TG_FILE_SIZE.by_url {
            if let MaybeLocalBlob::None = &maybe_local_blob {
                try_return_upload!(ctx.by_url());
            }
        }

        let local_blob = match maybe_local_blob {
            MaybeLocalBlob::None => ctx
                .download_blob_to_ram(MAX_TG_FILE_SIZE.by_multipart)
                .await?
                .upcast(),
            MaybeLocalBlob::Some(local_blob) => local_blob,
        };

        ctx.by_multipart(&local_blob).upload().await
    }

    fn file_kind(&self, tg_file_type: TgFileKind) -> TgUploadKindContext<'_> {
        TgUploadKindContext {
            base: self,
            tg_file_type,
        }
    }
}

enum MaybeLocalBlob {
    Some(LocalBlob<LocalBlobKind>),
    None,
}

#[derive(Debug)]
struct LocalBlob<B> {
    blob: B,
    size: u64,
}

#[derive(FromVariants)]
enum LocalBlobKind {
    Ram(bytes::Bytes),
    Disk(tempfile::TempPath),
}

impl<B> LocalBlob<B> {
    fn as_ref(&self) -> LocalBlob<&B> {
        LocalBlob {
            blob: &self.blob,
            size: self.size,
        }
    }

    fn upcast(self) -> LocalBlob<LocalBlobKind>
    where
        B: Into<LocalBlobKind>,
    {
        LocalBlob {
            blob: self.blob.into(),
            size: self.size,
        }
    }
}

impl LocalBlobKind {
    fn to_tg_input_file(&self) -> InputFile {
        match self {
            LocalBlobKind::Ram(bytes) => InputFile::memory(bytes.clone()),
            LocalBlobKind::Disk(file) => InputFile::file(file.to_path_buf()),
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
    async fn download_blob_to_ram(&self, max_size: u64) -> Result<LocalBlob<bytes::Bytes>> {
        self.download_blob_imp(reqwest::Response::bytes, max_size)
            .await
    }

    async fn download_blob_to_disk(&self, max_size: u64) -> Result<LocalBlob<tempfile::TempPath>> {
        self.download_blob_imp(reqwest::Response::read_to_temp_file, max_size)
            .await
    }

    async fn download_blob_imp<Payload, E, Fut>(
        &self,
        download: fn(reqwest::Response) -> Fut,
        max_size: u64,
    ) -> Result<LocalBlob<Payload>>
    where
        Fut: Future<Output = Result<Payload, E>> + Send,
        E: Into<Box<DynError>>,
    {
        let labels = DownloadLabels {
            blob_kind: <&'static str>::from(self.blob.repr.kind),
            tg_blob_kind: <&'static str>::from(self.tg_file_type),
        };

        let (downloaded, duration) = async {
            let (response, content_length) = self
                .http
                .get(self.blob.repr.download_url.clone())
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
            // so the workaround is using the content length information from the
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
                    self.blob.repr.download_url
                )
            })?;

            Ok::<_, crate::Error>(LocalBlob {
                blob: payload,
                size: content_length,
            })
        }
        .record_duration(blob_download_duration_seconds, labels)
        .with_duration_ok()
        .await?;

        blob_original_size_bytes(vec![]).record(downloaded.size as f64);

        info!(
            actual_size = %display::human_size(downloaded.size),
            duration = tracing_duration(duration),
            "Downloaded file"
        );

        Ok(downloaded)
    }

    fn method<'a>(&'a self, tg_upload_method: TgUploadMethod<'a>) -> TgUploadMethodContext<'a> {
        TgUploadMethodContext {
            base: self,
            tg_upload_method,
        }
    }

    fn by_url(&self) -> TgUploadMethodContext<'_> {
        self.method(TgUploadMethod::Url)
    }

    fn by_multipart<'a>(
        &'a self,
        downloaded: &'a LocalBlob<LocalBlobKind>,
    ) -> TgUploadMethodContext<'a> {
        self.method(TgUploadMethod::Multipart(downloaded.as_ref()))
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
            download_url = %self.blob.repr.download_url,
            blob_kind = %self.blob.repr.kind,
            blob_size = ?self.blob.repr.size,
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
        self.make_cached_blob(self.upload_file(self.input_file()).await?)
    }

    fn input_file(&self) -> InputFile {
        match &self.tg_upload_method {
            TgUploadMethod::Url => InputFile::url(self.blob.repr.download_url.clone()),
            TgUploadMethod::Multipart(local_blob) => local_blob.blob.to_tg_input_file(),
        }
        .file_name(self.blob.tg_file_name(self.post))
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
            blob_kind: <&'static str>::from(self.blob.repr.kind),
            tg_blob_kind: <&'static str>::from(&self.tg_upload_method),
            tg_upload_method: <&'static str>::from(self.tg_file_type),
        }
    }

    fn caption(&self) -> String {
        let core_caption = self.post.caption();
        let requested_by = self.requested_by.md_link();
        let via_method = match &self.tg_upload_method {
            TgUploadMethod::Url => "via URL",
            TgUploadMethod::Multipart(_) => "via multipart",
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
            id: file_meta.id.0,
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
                    expected: self.blob.repr.kind,
                }))
            }
        })
    }
}
