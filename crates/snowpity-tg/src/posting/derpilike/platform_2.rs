// use crate::posting::derpilike::api::{self, MediaId};
// use crate::posting::derpilike::{db, Config};
// use crate::posting::platform::prelude::*;
// use crate::prelude::*;
// use crate::Result;
// use async_trait::async_trait;
// use reqwest::Url;

// mod derpibooru {
//     use super::Derpilike;

//     struct Platform {
//         imp: super::Platform,
//     }

//     struct PlatformImp;

//     impl Derpilike for Platform {
//         const DISPLAY_NAME: &'static str = "Derpibooru";

//         fn parse_query(
//             query: &str,
//         ) -> super::ParseQueryResult<crate::posting::derpilike::api::MediaId> {
//             todo!()
//         }

//         fn imp(&self) -> &PlatformImp {
//             todo!()
//         }
//     }
// }

// mod ponerpics {
//     use super::Derpilike;

//     struct Ponerpics;

//     struct PlatformImp;

//     impl Derpilike for Ponerpics {
//         const DISPLAY_NAME: &'static str = "Ponerpics";

//         fn parse_query(
//             query: &str,
//         ) -> super::ParseQueryResult<crate::posting::derpilike::api::MediaId> {
//             todo!()
//         }

//         fn imp(&self) -> &PlatformImp {
//             todo!()
//         }
//     }
// }

// mod furbooru {
//     use super::Derpilike;

//     struct Furbooru;

//     struct PlatformImp;

//     impl Derpilike for Furbooru {
//         const DISPLAY_NAME: &'static str = "Furbooru";

//         fn parse_query(
//             query: &str,
//         ) -> super::ParseQueryResult<crate::posting::derpilike::api::MediaId> {
//             todo!()
//         }

//         fn imp(&self) -> &PlatformImp {
//             todo!()
//         }
//     }
// }

// struct PlatformImp;

// trait Derpilike {
//     // "Derpibooru"
//     const DISPLAY_NAME: &'static str;

//     // tg_{DB_NAME}_blob_cache
//     // const DB_NAME: &'static str;

//     // const ENDPOINT_URL: &'static str;

//     fn parse_query(query: &str) -> ParseQueryResult<MediaId>;

//     fn imp(&self) -> &PlatformImp;
// }
// /*
// Derpibooru
// Ponerpics
// Furbooru
// */
// struct Derpitools {
//     api: api::Client,
//     db: db::BlobCacheRepo,
// }

// pub(crate) struct Platform {
//     api: api::Client,
//     db: db::BlobCacheRepo,
// }

// impl PlatformTypes for Platform {
//     type PostId = MediaId;
//     type BlobId = ();
//     type RequestId = MediaId;
// }

// #[async_trait]
// impl<T: Derpilike> PlatformTrait for T {
//     type Config = Config;

//     const NAME: &'static str = T::DISPLAY_NAME;

//     fn new(params: PlatformParams<Config>) -> Self {
//         Self {
//             api: api::Client::new(params.config, params.http),
//             db: db::BlobCacheRepo::new(params.db),
//         }
//     }

//     fn parse_query(query: &str) -> ParseQueryResult<MediaId> {
//         Derpilike::parse_query(query)
//     }

//     async fn get_post(&self, media: MediaId) -> Result<Post<Self>> {
//         let media = self
//             .api
//             .get_media(media)
//             .instrument(info_span!("Fetching media meta from "))
//             .await?;

//         let authors = media.authors().map_collect(|author| Author {
//             web_url: author.web_url(),
//             kind: match author.kind {
//                 api::AuthorKind::Artist => None,
//                 api::AuthorKind::Editor => Some(AuthorKind::Editor),
//                 api::AuthorKind::Prompter => Some(AuthorKind::Prompter),
//             },
//             name: author.name,
//         });

//         let safety = media.safety_rating_tags().map(ToOwned::to_owned).collect();
//         let safety = if safety == ["safe"] {
//             SafetyRating::Sfw
//         } else {
//             SafetyRating::Nsfw { kinds: safety }
//         };

//         let dimensions = MediaDimensions {
//             width: media.width,
//             height: media.height,
//         };

//         let repr = best_tg_reprs(&media).map_collect(|(download_url, kind)| {
//             BlobRepr {
//                 dimensions: Some(dimensions),
//                 download_url,
//                 kind,
//                 // Sizes for images are ~good enough, although not always accurate,
//                 // but we don't know the size of MP4 equivalent for GIF or WEBM,
//                 // however those will often fit into the limit of uploading via direct URL.
//                 // Anyway, this is all not precise, so be it this way for now.
//                 size: BlobSize::Unknown,
//             }
//         });

//         let blob = MultiBlob { id: (), repr };

//         Ok(Post {
//             base: BasePost {
//                 id: media.id,
//                 authors,
//                 web_url: media.id.to_webpage_url(),
//                 safety,
//             },
//             blobs: vec![blob],
//         })
//     }

//     async fn get_cached_blobs(&self, media: MediaId) -> Result<Vec<CachedBlobId<Self>>> {
//         Ok(Vec::from_iter(
//             self.db
//                 .get(media)
//                 .with_duration_log("Reading the cache from the database")
//                 .await?
//                 .map(CachedBlobId::with_tg_file),
//         ))
//     }

//     async fn set_cached_blob(&self, media: MediaId, blob: CachedBlobId<Self>) -> Result {
//         self.db.set(media, blob.tg_file).await
//     }
// }

// impl DisplayInFileNameViaToString for api::MediaId {}

// /// URL of the media that best suits Telegram.
// ///
// /// Right now this is just the `view_url`, i.e. the original image representation.
// /// Best would be if derpibooru could generate the representation of an image for
// /// 2560x2560 pixels, but the biggest non-original representation is 1280x1024,
// /// according to philomena's [sources].
// ///
// /// This doesn't however guarantee the images will have top-notch quality (see [wiki]).
// /// The GIFs don't use the `passthrough` flag when they are converted to MP4,
// /// which means the FPS of the MP4 may be lower than the original GIF, so we
// /// are re-generating the MP4 on the fly ourselves.
// ///
// /// [wiki]: https://github.com/Veetaha/snowpity/wiki/Telegram-images-compression
// /// [sources]: https://github.com/philomena-dev/philomena/blob/743699c6afe38b20b23f866c2c1a590c86d6095e/lib/philomena/images/thumbnailer.ex#L16-L24
// fn best_tg_reprs(media: &api::Media) -> Vec<(Url, BlobKind)> {
//     match media.mime_type {
//         api::MimeType::ImageJpeg => vec![(media.view_url.clone(), BlobKind::ImageJpeg)],
//         api::MimeType::ImagePng => vec![(media.view_url.clone(), BlobKind::ImagePng)],
//         api::MimeType::ImageSvgXml => vec![(media.view_url.clone(), BlobKind::ImageSvg)],
//         api::MimeType::ImageGif => {
//             vec![
//                 // First of all try to get an existing MP4 representation for the GIF
//                 (media.unwrap_mp4_url(), BlobKind::AnimationMp4),
//                 // If there is no MP4 representation, then generate it on the fly
//                 // from the original GIF file
//                 (media.view_url.clone(), BlobKind::AnimationGif),
//             ]
//         }
//         api::MimeType::VideoWebm => vec![(media.unwrap_mp4_url(), BlobKind::VideoMp4)],
//     }
// }

// #[cfg(test)]
// mod tests {
//     use expect_test::expect;

//     #[test]
//     fn smoke() {
//         use crate::posting::platform::tests::assert_parse_query as test;
//         test(
//             "derpibooru.org/123/",
//             expect!["derpibooru.org:Derpibooru(MediaId(123))"],
//         );
//         test(
//             "derpibooru.org/123",
//             expect!["derpibooru.org:Derpibooru(MediaId(123))"],
//         );
//         test(
//             "derpibooru.org/images/123",
//             expect!["derpibooru.org/images:Derpibooru(MediaId(123))"],
//         );
//         test(
//             "derpibooru.org/images/123/",
//             expect!["derpibooru.org/images:Derpibooru(MediaId(123))"],
//         );
//         test(
//             "https://derpicdn.net/img/2022/12/17/3008328/large.jpg",
//             expect!["derpicdn.net/img:Derpibooru(MediaId(3008328))"],
//         );
//         test(
//             "https://derpicdn.net/img/view/2022/12/17/3008328.jpg",
//             expect!["derpicdn.net/img/view:Derpibooru(MediaId(3008328))"],
//         );
//         test(
//             "https://derpicdn.net/img/download/2022/12/28/3015836__safe_artist-colon-shadowreindeer_foo.jpg",
//             expect!["derpicdn.net/img/download:Derpibooru(MediaId(3015836))"]
//         );
//     }
// }
