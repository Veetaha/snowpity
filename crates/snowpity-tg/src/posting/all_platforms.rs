use super::derpilike::{derpibooru, ponerpics};
use super::platform::prelude::*;
use super::{deviant_art, twitter};
use crate::prelude::*;
use crate::Result;
use assert_matches::assert_matches;

macro_rules! def_all_platforms {
    (
        $([$platform:ident, $Platform:ident]),* $(,)?
    ) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub(crate) enum RequestId {
            $( $Platform(<$platform::Platform as PlatformTypes>::RequestId), )*
        }

        #[derive(Clone, PartialEq, Eq, Hash, Debug)]
        pub(crate) enum PostId {
            $( $Platform(<$platform::Platform as PlatformTypes>::PostId), )*
        }

        #[derive(Clone, PartialEq, Eq, Hash, Debug)]
        pub(crate) enum BlobId {
            $( $Platform(<$platform::Platform as PlatformTypes>::BlobId), )*
        }

        impl DisplayInFileName for PostId {
            fn display_in_file_name(&self) -> Option<String> {
                match self {
                    $( Self::$Platform(id) => id.display_in_file_name(), )*
                }
            }
        }

        impl DisplayInFileName for BlobId {
            fn display_in_file_name(&self) -> Option<String> {
                match self {
                    $( Self::$Platform(id) => id.display_in_file_name(), )*
                }
            }
        }

        impl PostId {
            /// Name of the posting platform that hosts the post.
            pub(crate) fn platform_name(&self) -> &'static str {
                match self {
                    $( Self::$Platform(_) => <$platform::Platform as PlatformTrait>::NAME, )*
                }
            }
        }

        pub(crate) struct Config {
            $( pub(crate) $platform: <$platform::Platform as PlatformTrait>::Config, )*
        }

        impl Config {
            pub(crate) fn load_or_panic() -> Config {
                Self {
                    $(
                        $platform: crate::config::from_env_or_panic(
                            <$platform::Platform as PlatformTrait>::Config::ENV_PREFIX
                        ),
                    )*
                }
            }
        }

        pub(crate) struct AllPlatforms {
            $( $platform: $platform::Platform, )*
        }

        impl AllPlatforms {
            pub(crate) fn new(params: PlatformParams<Config>) -> Self {
                Self {
                    $(
                        $platform: <$platform::Platform as PlatformTrait>::new(PlatformParams {
                            config: params.config.$platform,
                            http: params.http.clone(),
                            db: params.db.clone(),
                        }),
                    )*
                }
            }

            pub(crate) async fn get_post(&self, id: RequestId) -> Result<Post> {
                Ok(match id {
                    $(
                        RequestId::$Platform(id) => {
                            let post = self.$platform.get_post(id).await?;
                            let blobs = post.blobs.map_collect(|blob| {
                                let MultiBlob { repr, id } = blob;
                                MultiBlob { repr, id: BlobId::$Platform(id) }
                            });

                            let BasePost {
                                id,
                                authors,
                                web_url,
                                safety,
                            } = post.base;

                            let base = BasePost {
                                id: PostId::$Platform(id),
                                authors,
                                web_url,
                                safety,
                            };

                            Post { base, blobs }
                        }
                    )*
                })
            }

            pub(crate) async fn get_cached_blobs(
                &self,
                request: RequestId,
            ) -> Result<Vec<CachedBlobId>> {
                Ok(match request {
                    $(
                        RequestId::$Platform(request) => {
                            self
                                .$platform
                                .get_cached_blobs(request)
                                .await?
                                .map_collect(|blob| CachedBlobId {
                                    id: BlobId::$Platform(blob.id),
                                    tg_file: blob.tg_file,
                                })
                        }
                    )*
                })
            }

            pub(crate) async fn set_cached_blob(
                &self,
                post: PostId,
                blob: CachedBlobId<Self>,
            ) -> Result {
                match post {
                    $(
                        PostId::$Platform(post) => {
                            let id = assert_matches!(blob.id, BlobId::$Platform(blob) => blob);
                            let blob = CachedBlobId {
                                id,
                                tg_file: blob.tg_file,
                            };
                            self.$platform.set_cached_blob(post, blob).await
                        }
                    )*
                }
            }
        }

        pub(crate) fn parse_query(input: &str) -> ParseQueryResult<RequestId> {
            let input = input.trim();

            $(
                if let Some((platform, id)) = <$platform::Platform as PlatformTrait>::parse_query(input) {
                    return Some((platform, RequestId::$Platform(id)));
                }
            )*

            None
        }
    }
}

def_all_platforms! {
    [derpibooru, Derpibooru],
    [twitter, Twitter],
    [deviant_art, DeviantArt],
    [ponerpics, Ponerpics]
}

// #[derive(Debug, Clone, PartialEq, Eq, Hash)]
// pub(crate) enum RequestId {
//     Derpibooru(<derpibooru::Platform as PlatformTypes>::RequestId),
//     Twitter(<twitter::Platform as PlatformTypes>::RequestId),
//     DeviantArt(<deviant_art::Platform as PlatformTypes>::RequestId),
// }
// #[derive(Clone, PartialEq, Eq, Hash, Debug)]
// pub(crate) enum PostId {
//     Derpibooru(<derpibooru::Platform as PlatformTypes>::PostId),
//     Twitter(<twitter::Platform as PlatformTypes>::PostId),
//     DeviantArt(<deviant_art::Platform as PlatformTypes>::PostId),
// }
// #[derive(Clone, PartialEq, Eq, Hash, Debug)]
// pub(crate) enum BlobId {
//     Derpibooru(<derpibooru::Platform as PlatformTypes>::BlobId),
//     Twitter(<twitter::Platform as PlatformTypes>::BlobId),
//     DeviantArt(<deviant_art::Platform as PlatformTypes>::BlobId),
// }
// impl DisplayInFileName for PostId {
//     fn display_in_file_name(&self) -> Option<String> {
//         match self {
//             Self::Derpibooru(id) => id.display_in_file_name(),
//             Self::Twitter(id) => id.display_in_file_name(),
//             Self::DeviantArt(id) => id.display_in_file_name(),
//         }
//     }
// }
// impl DisplayInFileName for BlobId {
//     fn display_in_file_name(&self) -> Option<String> {
//         match self {
//             Self::Derpibooru(id) => id.display_in_file_name(),
//             Self::Twitter(id) => id.display_in_file_name(),
//             Self::DeviantArt(id) => id.display_in_file_name(),
//         }
//     }
// }
// impl PostId {
//     #[doc = r" Name of the posting platform that hosts the post."]
//     pub(crate) fn platform_name(&self) -> &'static str {
//         match self {
//             Self::Derpibooru(_) => <derpibooru::Platform as PlatformTrait>::NAME,
//             Self::Twitter(_) => <twitter::Platform as PlatformTrait>::NAME,
//             Self::DeviantArt(_) => <deviant_art::Platform as PlatformTrait>::NAME,
//         }
//     }
// }
// pub(crate) struct Config {
//     pub(crate) derpibooru: <derpibooru::Platform as PlatformTrait>::Config,
//     pub(crate) twitter: <twitter::Platform as PlatformTrait>::Config,
//     pub(crate) deviant_art: <deviant_art::Platform as PlatformTrait>::Config,
// }
// impl Config {
//     pub(crate) fn load_or_panic() -> Config {
//         Self {
//             derpibooru: crate::config::from_env_or_panic(
//                 <derpibooru::Platform as PlatformTrait>::Config::ENV_PREFIX,
//             ),
//             twitter: crate::config::from_env_or_panic(
//                 <twitter::Platform as PlatformTrait>::Config::ENV_PREFIX,
//             ),
//             deviant_art: crate::config::from_env_or_panic(
//                 <deviant_art::Platform as PlatformTrait>::Config::ENV_PREFIX,
//             ),
//         }
//     }
// }
// pub(crate) struct AllPlatforms {
//     derpibooru: derpibooru::Platform,
//     twitter: twitter::Platform,
//     deviant_art: deviant_art::Platform,
// }
// impl AllPlatforms {
//     pub(crate) fn new(params: PlatformParams<Config>) -> Self {
//         Self {
//             derpibooru: <derpibooru::Platform as PlatformTrait>::new(PlatformParams {
//                 config: params.config.derpibooru,
//                 http: params.http.clone(),
//                 db: params.db.clone(),
//             }),
//             twitter: <twitter::Platform as PlatformTrait>::new(PlatformParams {
//                 config: params.config.twitter,
//                 http: params.http.clone(),
//                 db: params.db.clone(),
//             }),
//             deviant_art: <deviant_art::Platform as PlatformTrait>::new(PlatformParams {
//                 config: params.config.deviant_art,
//                 http: params.http.clone(),
//                 db: params.db.clone(),
//             }),
//         }
//     }
//     pub(crate) async fn get_post(&self, id: RequestId) -> Result<Post> {
//         Ok(match id {
//             RequestId::Derpibooru(id) => {
//                 let post = self.derpibooru.get_post(id).await?;
//                 let blobs = post.blobs.map_collect(|blob| {
//                     let MultiBlob { repr, id } = blob;
//                     MultiBlob {
//                         repr,
//                         id: BlobId::Derpibooru(id),
//                     }
//                 });
//                 let BasePost {
//                     id,
//                     authors,
//                     web_url,
//                     safety,
//                 } = post.base;
//                 let base = BasePost {
//                     id: PostId::Derpibooru(id),
//                     authors,
//                     web_url,
//                     safety,
//                 };
//                 Post { base, blobs }
//             }
//             RequestId::Twitter(id) => {
//                 let post = self.twitter.get_post(id).await?;
//                 let blobs = post.blobs.map_collect(|blob| {
//                     let MultiBlob { repr, id } = blob;
//                     MultiBlob {
//                         repr,
//                         id: BlobId::Twitter(id),
//                     }
//                 });
//                 let BasePost {
//                     id,
//                     authors,
//                     web_url,
//                     safety,
//                 } = post.base;
//                 let base = BasePost {
//                     id: PostId::Twitter(id),
//                     authors,
//                     web_url,
//                     safety,
//                 };
//                 Post { base, blobs }
//             }
//             RequestId::DeviantArt(id) => {
//                 let post = self.deviant_art.get_post(id).await?;
//                 let blobs = post.blobs.map_collect(|blob| {
//                     let MultiBlob { repr, id } = blob;
//                     MultiBlob {
//                         repr,
//                         id: BlobId::DeviantArt(id),
//                     }
//                 });
//                 let BasePost {
//                     id,
//                     authors,
//                     web_url,
//                     safety,
//                 } = post.base;
//                 let base = BasePost {
//                     id: PostId::DeviantArt(id),
//                     authors,
//                     web_url,
//                     safety,
//                 };
//                 Post { base, blobs }
//             }
//         })
//     }
//     pub(crate) async fn get_cached_blobs(&self, request: RequestId) -> Result<Vec<CachedBlobId>> {
//         Ok(match request {
//             RequestId::Derpibooru(request) => self
//                 .derpibooru
//                 .get_cached_blobs(request)
//                 .await?
//                 .map_collect(|blob| CachedBlobId {
//                     id: BlobId::Derpibooru(blob.id),
//                     tg_file: blob.tg_file,
//                 }),
//             RequestId::Twitter(request) => self
//                 .twitter
//                 .get_cached_blobs(request)
//                 .await?
//                 .map_collect(|blob| CachedBlobId {
//                     id: BlobId::Twitter(blob.id),
//                     tg_file: blob.tg_file,
//                 }),
//             RequestId::DeviantArt(request) => self
//                 .deviant_art
//                 .get_cached_blobs(request)
//                 .await?
//                 .map_collect(|blob| CachedBlobId {
//                     id: BlobId::DeviantArt(blob.id),
//                     tg_file: blob.tg_file,
//                 }),
//         })
//     }
//     pub(crate) async fn set_cached_blob(&self, post: PostId, blob: CachedBlobId<Self>) -> Result {
//         match post {
//             PostId::Derpibooru(post) => {
//                 let id = assert_matches!(blob.id,BlobId::Derpibooru(blob) => blob);
//                 let blob = CachedBlobId {
//                     id,
//                     tg_file: blob.tg_file,
//                 };
//                 self.derpibooru.set_cached_blob(post, blob).await
//             }
//             PostId::Twitter(post) => {
//                 let id = assert_matches!(blob.id,BlobId::Twitter(blob) => blob);
//                 let blob = CachedBlobId {
//                     id,
//                     tg_file: blob.tg_file,
//                 };
//                 self.twitter.set_cached_blob(post, blob).await
//             }
//             PostId::DeviantArt(post) => {
//                 let id = assert_matches!(blob.id,BlobId::DeviantArt(blob) => blob);
//                 let blob = CachedBlobId {
//                     id,
//                     tg_file: blob.tg_file,
//                 };
//                 self.deviant_art.set_cached_blob(post, blob).await
//             }
//         }
//     }
// }
// pub(crate) fn parse_query(input: &str) -> ParseQueryResult<RequestId> {
//     let input = input.trim();
//     if let Some((platform, id)) = <derpibooru::Platform as PlatformTrait>::parse_query(input) {
//         return Some((platform, RequestId::Derpibooru(id)));
//     }
//     if let Some((platform, id)) = <twitter::Platform as PlatformTrait>::parse_query(input) {
//         return Some((platform, RequestId::Twitter(id)));
//     }
//     if let Some((platform, id)) = <deviant_art::Platform as PlatformTrait>::parse_query(input) {
//         return Some((platform, RequestId::DeviantArt(id)));
//     }
//     None
// }

impl PlatformTypes for AllPlatforms {
    type RequestId = RequestId;
    type PostId = PostId;
    type BlobId = BlobId;
}
