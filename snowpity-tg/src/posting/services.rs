use super::model::TgFileMeta;
use super::{
    derpi, twitter, ConfigTrait, DisplayInFileName, DistinctPostMetaTrait, ParseQueryResult,
    ResolvePostResult, ServiceParams, ServiceTrait,
};
use crate::Result;
use assert_matches::assert_matches;

macro_rules! def_service_types {
    (
        $([$service:ident, $Service:ident]),* $(,)?
    ) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, from_variants::FromVariants)]
        pub(crate) enum RequestId {
            $( $Service(<$service::Service as ServiceTrait>::RequestId), )*
        }

        #[derive(Clone, PartialEq, Eq, Hash, Debug, from_variants::FromVariants)]
        pub(crate) enum PostId {
            $( $Service(<$service::Service as ServiceTrait>::PostId), )*
        }

        #[derive(Clone, PartialEq, Eq, Hash, Debug, from_variants::FromVariants)]
        pub(crate) enum BlobId {
            $( $Service(<$service::Service as ServiceTrait>::BlobId), )*
        }

        impl DisplayInFileName for PostId {
            fn display_in_file_name(&self) -> Option<String> {
                match self {
                    $( Self::$Service(id) => id.display_in_file_name(), )*
                }
            }
        }

        impl DisplayInFileName for BlobId {
            fn display_in_file_name(&self) -> Option<String> {
                match self {
                    $( Self::$Service(id) => id.display_in_file_name(), )*
                }
            }
        }

        #[derive(Clone)]
        pub(crate) enum DistinctPostMeta {
            $( $Service(<$service::Service as ServiceTrait>::DistinctPostMeta), )*
        }

        impl DistinctPostMeta {
            /// Name of the posting platform that hosts the post.
            pub(crate) fn platform_name(&self) -> &'static str {
                match self {
                    $( Self::$Service(_) => <$service::Service as ServiceTrait>::NAME, )*
                }
            }
        }

        impl DistinctPostMetaTrait for DistinctPostMeta {
            fn nsfw_ratings(&self) -> Vec<&str> {
                match &self {
                    $( Self::$Service(distinct) => distinct.nsfw_ratings(), )*
                }
            }
        }

        pub(crate) struct Config {
            $( $service: <$service::Service as ServiceTrait>::Config, )*
        }

        impl Config {
            pub(crate) fn load_or_panic() -> Config {
                Self {
                    $(
                        $service: crate::config::from_env_or_panic(
                            <$service::Service as ServiceTrait>::Config::ENV_PREFIX
                        ),
                    )*
                }
            }
        }

        pub(crate) struct Services {
            $( $service: $service::Service, )*
        }

        impl Services {
            pub(crate) fn new(params: ServiceParams<Config>) -> Self {
                Self {
                    $(
                        $service: <$service::Service as ServiceTrait>::new(ServiceParams {
                            config: params.config.$service,
                            http: params.http.clone(),
                            db: params.db.clone(),
                        }),
                    )*
                }
            }

            pub(crate) fn parse_query(input: &str) -> ParseQueryResult<'_, RequestId> {
                let input = input.trim();

                $(
                    if let Some(result) = <$service::Service as ServiceTrait>::parse_query(input) {
                        return result.into();
                    }
                )*

                None
            }

            pub(crate) async fn resolve_post(&self, request: RequestId) -> ResolvePostResult {
                match request {
                    $( RequestId::$Service(request) => self.$service.resolve_post(request).await, )*
                }
            }

            pub(crate) async fn set_cached_blob(
                &self,
                post: PostId,
                blob: BlobId,
                tg_file_meta: TgFileMeta
            ) -> Result {
                match post {
                    $(
                        PostId::$Service(post) => {
                            let blob = assert_matches!(blob, BlobId::$Service(blob) => blob);
                            self.$service.set_cached_blob(post, blob, tg_file_meta).await
                        }
                    )*
                }
            }
        }
    }
}

def_service_types! {
    [derpi, Derpi],
    [twitter, Twitter],
}
