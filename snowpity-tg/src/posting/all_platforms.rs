use super::platform::prelude::*;
use super::{derpi, twitter};
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

        #[derive(Clone)]
        pub(crate) enum DistinctPostMeta {
            $( $Platform(<$platform::Platform as PlatformTypes>::DistinctPostMeta), )*
        }

        impl DistinctPostMeta {
            /// Name of the posting platform that hosts the post.
            pub(crate) fn platform_name(&self) -> &'static str {
                match self {
                    $( Self::$Platform(_) => <$platform::Platform as PlatformTrait>::NAME, )*
                }
            }
        }

        impl DistinctPostMetaTrait for DistinctPostMeta {
            fn nsfw_ratings(&self) -> Vec<&str> {
                match &self {
                    $( Self::$Platform(distinct) => distinct.nsfw_ratings(), )*
                }
            }
        }

        pub(crate) struct Config {
            $( $platform: <$platform::Platform as PlatformTrait>::Config, )*
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
                            let blobs = post
                                .blobs
                                .into_iter()
                                .map(|blob| {
                                    let Blob {
                                        id,
                                        kind,
                                        dimensions,
                                        size,
                                        download_url,
                                    } = blob;

                                    Blob {
                                        id: BlobId::$Platform(id),
                                        kind,
                                        dimensions,
                                        size,
                                        download_url,
                                    }
                                })
                                .collect();

                            let BasePost {
                                id,
                                authors,
                                web_url,
                                distinct,
                            } = post.base;

                            let base = BasePost {
                                id: PostId::$Platform(id),
                                authors,
                                web_url,
                                distinct: DistinctPostMeta::$Platform(distinct),
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
                                .get_cached_blobs(request).await?
                                .into_iter()
                                .map(|blob| CachedBlobId {
                                    id: BlobId::$Platform(blob.id),
                                    tg_file: blob.tg_file,
                                })
                                .collect()
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

        pub(crate) fn parse_query(input: &str) -> ParseQueryResult<'_, RequestId> {
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
    [derpi, Derpi],
    [twitter, Twitter],
}

impl PlatformTypes for AllPlatforms {
    type RequestId = RequestId;
    type PostId = PostId;
    type BlobId = BlobId;
    type DistinctPostMeta = DistinctPostMeta;
}
