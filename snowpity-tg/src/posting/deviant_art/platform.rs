use crate::posting::deviant_art::api::{self, DeviationId};
use crate::posting::deviant_art::{db, Config};
use crate::posting::platform::prelude::*;
use crate::prelude::*;
use crate::Result;
use async_trait::async_trait;

pub(crate) struct Platform {
    api: api::Client,
    db: db::BlobCacheRepo,
}

impl PlatformTypes for Platform {
    type PostId = DeviationId;
    type BlobId = ();
    type RequestId = DeviationId;
}

#[async_trait]
impl PlatformTrait for Platform {
    type Config = Config;

    const NAME: &'static str = "DeviantArt";

    fn new(params: PlatformParams<Config>) -> Self {
        Self {
            api: api::Client::new(params.config, params.http),
            db: db::BlobCacheRepo::new(params.db),
        }
    }

    fn parse_query(query: &str) -> ParseQueryResult<DeviationId> {
        // Example:
        // https://miltvain.deviantart.com/art/Twilight-magic-418078970
        'first_try: {
            if let Some((_, host_prefix, author, art, id)) = parse_with_regexes!(
                query,
                r"(?:https://)?(www\.)?(.+?)\.deviantart\.com/art/(.+)-(\d+)"
            ) {
                // www.deviantart.com links still match the regex here;
                // We could ignore them here if there was lookbehind support in regexes,
                // but there isn't for security and perf. reasons, so we have
                // this workaround instead.
                if author == "www" {
                    break 'first_try;
                }

                let id = id.parse().ok()?;
                let art = art.to_owned();
                let author = author.to_owned();


                let host = format!("{host_prefix}{{author}}.deviantart.com");
                return Some((host, DeviationId::Full { author, art, id }));
            }
        }

        if let Some((_, host, author, art, id)) = parse_with_regexes!(
            query,
            r"((?:www\.)?deviantart\.com)/(?:(.+)/)?art/(.+)-(\d+)"
        ) {
            let id = id.parse().ok()?;
            let art = art.to_owned();

            if author.is_empty() {
                return Some((host.into(), DeviationId::ArtAndId { art, id }));
            }

            let author = author.to_owned();

            return Some((host.into(), DeviationId::Full { author, art, id }));
        }

        let (_, host, id) = parse_with_regexes!(
            query,
            r"(deviantart\.com/deviation)/(\d+)",
            r"(view.deviantart\.com)/(\d+)",
        )?;

        Some((host.into(), DeviationId::Id(id.parse().ok()?)))
    }

    async fn get_post(&self, deviation: DeviationId) -> Result<Post<Self>> {
        let oembed = self
            .api
            .get_oembed(deviation.clone())
            .instrument(info_span!("Fetching media meta from DeviantArt"))
            .await?;

        let author = Author {
            web_url: oembed.author_url,
            kind: None,
            name: oembed.author_name,
        };

        let dimensions = MediaDimensions {
            width: oembed.width,
            height: oembed.height,
        };

        let file_extension = oembed.url.file_extension().ok_or_else(|| {
            crate::fatal!(
                "DeviantArt returned a URL without a file extension: {}",
                oembed.url
            )
        })?;

        let (kind, size) = match file_extension {
            "png" => (BlobKind::ImagePng, BlobSize::Unknown),
            "jpg" => (BlobKind::ImageJpeg, BlobSize::Unknown),
            "gif" => (BlobKind::AnimationGif, BlobSize::Unknown),
            _ => {
                return Err(crate::fatal!(
                    "Unsupported DeviantArt file extension: `{file_extension}`",
                ))
            }
        };

        let blob = MultiBlob::from_single(BlobRepr {
            dimensions,
            // https://images-wixmp-ed30a86b8c4ca887773594c2.wixmp.com/f/19028192-11ac-4c82-a0eb-d3491a319677/dforc14-2936d279-3f9d-4a00-b82f-a37173292f45.png/v1/fill/w_956,h_836,q_70,strp/snowy_day_by_vinilyart_dforc14-pre.jpg?token=eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ1cm46YXBwOjdlMGQxODg5ODIyNjQzNzNhNWYwZDQxNWVhMGQyNmUwIiwiaXNzIjoidXJuOmFwcDo3ZTBkMTg4OTgyMjY0MzczYTVmMGQ0MTVlYTBkMjZlMCIsIm9iaiI6W1t7ImhlaWdodCI6Ijw9MTEyMCIsInBhdGgiOiJcL2ZcLzE5MDI4MTkyLTExYWMtNGM4Mi1hMGViLWQzNDkxYTMxOTY3N1wvZGZvcmMxNC0yOTM2ZDI3OS0zZjlkLTRhMDAtYjgyZi1hMzcxNzMyOTJmNDUucG5nIiwid2lkdGgiOiI8PTEyODAifV1dLCJhdWQiOlsidXJuOnNlcnZpY2U6aW1hZ2Uub3BlcmF0aW9ucyJdfQ.WjgY1dlY68TO2u7JbiSBHERuu7mM7YuA33nwN4fBVPU

            // 1436x1256px
            // original:
            // https://wixmp-ed30a86b8c4ca887773594c2.wixmp.com/f/19028192-11ac-4c82-a0eb-d3491a319677/dforc14-2936d279-3f9d-4a00-b82f-a37173292f45.png?token=eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ1cm46YXBwOjdlMGQxODg5ODIyNjQzNzNhNWYwZDQxNWVhMGQyNmUwIiwiaXNzIjoidXJuOmFwcDo3ZTBkMTg4OTgyMjY0MzczYTVmMGQ0MTVlYTBkMjZlMCIsImV4cCI6MTY3NTgyMDY2OCwiaWF0IjoxNjc1ODIwMDU4LCJqdGkiOiI2M2UyZmMyNDgzYTFkIiwib2JqIjpbW3sicGF0aCI6IlwvZlwvMTkwMjgxOTItMTFhYy00YzgyLWEwZWItZDM0OTFhMzE5Njc3XC9kZm9yYzE0LTI5MzZkMjc5LTNmOWQtNGEwMC1iODJmLWEzNzE3MzI5MmY0NS5wbmcifV1dLCJhdWQiOlsidXJuOnNlcnZpY2U6ZmlsZS5kb3dubG9hZCJdfQ.FOQqUhYP8F8o-rfexgLhLwaQUkITAG-oLFASm1EI_UU&filename=snowy_day_by_vinilyart_dforc14.png

            // https://images-wixmp-ed30a86b8c4ca887773594c2.wixmp.com/f/19028192-11ac-4c82-a0eb-d3491a319677/dforc14-2936d279-3f9d-4a00-b82f-a37173292f45.png/v1/fit/w_1280,h_1120,q_100/snowy_day_by_vinilyart_dforc14-pre.jpg?token=eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ1cm46YXBwOjdlMGQxODg5ODIyNjQzNzNhNWYwZDQxNWVhMGQyNmUwIiwiaXNzIjoidXJuOmFwcDo3ZTBkMTg4OTgyMjY0MzczYTVmMGQ0MTVlYTBkMjZlMCIsIm9iaiI6W1t7ImhlaWdodCI6Ijw9MTEyMCIsInBhdGgiOiJcL2ZcLzE5MDI4MTkyLTExYWMtNGM4Mi1hMGViLWQzNDkxYTMxOTY3N1wvZGZvcmMxNC0yOTM2ZDI3OS0zZjlkLTRhMDAtYjgyZi1hMzcxNzMyOTJmNDUucG5nIiwid2lkdGgiOiI8PTEyODAifV1dLCJhdWQiOlsidXJuOnNlcnZpY2U6aW1hZ2Uub3BlcmF0aW9ucyJdfQ.WjgY1dlY68TO2u7JbiSBHERuu7mM7YuA33nwN4fBVPU

            // https://images-wixmp-ed30a86b8c4ca887773594c2.wixmp.com/f/19028192-11ac-4c82-a0eb-d3491a319677/dforc14-2936d279-3f9d-4a00-b82f-a37173292f45.png?token=eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ1cm46YXBwOjdlMGQxODg5ODIyNjQzNzNhNWYwZDQxNWVhMGQyNmUwIiwiaXNzIjoidXJuOmFwcDo3ZTBkMTg4OTgyMjY0MzczYTVmMGQ0MTVlYTBkMjZlMCIsIm9iaiI6W1t7ImhlaWdodCI6Ijw9MTEyMCIsInBhdGgiOiJcL2ZcLzE5MDI4MTkyLTExYWMtNGM4Mi1hMGViLWQzNDkxYTMxOTY3N1wvZGZvcmMxNC0yOTM2ZDI3OS0zZjlkLTRhMDAtYjgyZi1hMzcxNzMyOTJmNDUucG5nIiwid2lkdGgiOiI8PTEyODAifV1dLCJhdWQiOlsidXJuOnNlcnZpY2U6aW1hZ2Uub3BlcmF0aW9ucyJdfQ.WjgY1dlY68TO2u7JbiSBHERuu7mM7YuA33nwN4fBVPU

            // https://images-wixmp-ed30a86b8c4ca887773594c2.wixmp.com/f/aa14a22e-70c1-4301-b452-36b07958ef14/dcnz8bf-d2eb40a7-f56d-43c7-b3f1-f14e0f970380.png?token=eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ1cm46YXBwOjdlMGQxODg5ODIyNjQzNzNhNWYwZDQxNWVhMGQyNmUwIiwiaXNzIjoidXJuOmFwcDo3ZTBkMTg4OTgyMjY0MzczYTVmMGQ0MTVlYTBkMjZlMCIsIm9iaiI6W1t7InBhdGgiOiJcL2ZcL2FhMTRhMjJlLTcwYzEtNDMwMS1iNDUyLTM2YjA3OTU4ZWYxNFwvZGNuejhiZi1kMmViNDBhNy1mNTZkLTQzYzctYjNmMS1mMTRlMGY5NzAzODAucG5nIn1dXSwiYXVkIjpbInVybjpzZXJ2aWNlOmZpbGUuZG93bmxvYWQiXX0.IEdXedrPOPybdU96M1JbNggjOePbFISSvItHam-F2Zg

            // TODO: select best URL
            // Example of the image that displays in original size in browser:
            // https://www.deviantart.com/freeedon/art/Cloudsdale-765869019
            // Example of this image that fits into 2560 square:
            //
            // https://images-wixmp-ed30a86b8c4ca887773594c2.wixmp.com/intermediary/f/
            // aa14a22e-70c1-4301-b452-36b07958ef14/dcnz8bf-d2eb40a7-f56d-43c7-b3f1-f14e0f970380.png
            // /v1/fit/w_2560,h_2560,bl,q_100/cloudsdale_by_freeedon_dcnz8bf.jpg
            //
            // See https://gist.github.com/micycle1/735006a338e4bea1a9c06377610886e7
            // for instructions from someone who reverse-engineered this
            //
            // GIF that is returned directly with .gif URL:
            // https://www.deviantart.com/negasun/art/Colgate-animated-gif-suggestive-655281025
            //
            // GIF that is returned directly with .gif/.jpg URL:
            // https://www.deviantart.com/yoshigreenwater/art/Door-Dash-Animated-854611048
            //
            // GIF that is not returned with a direct image URL:
            // https://www.deviantart.com/deannart/art/Re-upload-INNOCENCE-MOV-350237566

            // Example:
            // https://images-wixmp-ed30a86b8c4ca887773594c2.wixmp.com/i/
            // 10be6721-6c5e-4882-abce-c8ef4ec121f7/d5sit1a-ddf43555-a931-4bf1-a900-32bdde99097d.jpg/
            // v1/fit/w_300,h_720,q_70,strp/_re_upload__innocence_mov_by_deannart_d5sit1a-300w.jpg

            // https://images-wixmp-ed30a86b8c4ca887773594c2.wixmp.com/i/10be6721-6c5e-4882-abce-c8ef4ec121f7/d5sit1a-ddf43555-a931-4bf1-a900-32bdde99097d.jpg/v1/fit/w_300,h_720,q_70,strp/_re_upload__innocence_mov_by_deannart_d5sit1a-300w.jpg

            // https://wixmp-ed30a86b8c4ca887773594c2.wixmp.com/v/mp4/10be6721-6c5e-4882-abce-c8ef4ec121f7/d5sit1a-4f096690-c76d-4f63-8337-b67b978ac5cd.700p.3abd6dff632b4fb2a16dcd8aa8cb48e5.mp4
            download_url: oembed.url,
            kind,
            // Sizes for images are ~good enough, although not always accurate,
            // but we don't know the size of MP4 equivalent for GIF or WEBM,
            // however those will often fit into the limit of uploading via direct URL.
            size,
        });

        let safety = match oembed.safety {
            Some(api::Safety::Nonadult) => SafetyRating::Sfw,
            Some(api::Safety::Adult) => SafetyRating::nsfw(),
            Some(api::Safety::Other(other)) => {
                warn!(rating = %other, "Faced an unknown DeviantArt safety rating");
                SafetyRating::Nsfw { kinds: vec![other] }
            }
            None => SafetyRating::nsfw(),
        };

        Ok(Post {
            base: BasePost {
                web_url: deviation.to_canonical_url(),
                id: deviation,
                authors: <_>::from_iter([author]),
                safety,
            },
            blobs: vec![blob],
        })
    }

    async fn get_cached_blobs(&self, deviation: DeviationId) -> Result<Vec<CachedBlobId<Self>>> {
        Ok(Vec::from_iter(
            self.db
                .get(deviation.numeric())
                .with_duration_log("Reading the cache from the database")
                .await?
                .map(CachedBlobId::with_tg_file),
        ))
    }

    async fn set_cached_blob(&self, deviation: DeviationId, blob: CachedBlobId<Self>) -> Result {
        self.db.set(deviation.numeric(), blob.tg_file).await
    }
}

impl DisplayInFileName for api::DeviationId {
    fn display_in_file_name(&self) -> Option<String> {
        Some(self.numeric().to_string())
    }
}

#[cfg(test)]
mod tests {
    use expect_test::expect;

    #[test]
    fn parsing_author_in_domain() {
        use crate::posting::platform::tests::assert_parse_query as test;
        test(
            "www.miltvain.deviantart.com/art/Twilight-magic-418078970",
            expect![[r#"
                www.{author}.deviantart.com:DeviantArt(
                    Full {
                        author: "miltvain",
                        art: "Twilight-magic",
                        id: DeviationNumericId(
                            418078970,
                        ),
                    },
                )"#]],
        );
        test(
            "miltvain.deviantart.com/art/Twilight-magic-418078970",
            expect![[r#"
                {author}.deviantart.com:DeviantArt(
                    Full {
                        author: "miltvain",
                        art: "Twilight-magic",
                        id: DeviationNumericId(
                            418078970,
                        ),
                    },
                )"#]],
        );
        test(
            "https://www.miltvain.deviantart.com/art/Twilight-magic-418078970",
            expect![[r#"
                www.{author}.deviantart.com:DeviantArt(
                    Full {
                        author: "miltvain",
                        art: "Twilight-magic",
                        id: DeviationNumericId(
                            418078970,
                        ),
                    },
                )"#]],
        );
        test(
            "https://miltvain.deviantart.com/art/Twilight-magic-418078970",
            expect![[r#"
                {author}.deviantart.com:DeviantArt(
                    Full {
                        author: "miltvain",
                        art: "Twilight-magic",
                        id: DeviationNumericId(
                            418078970,
                        ),
                    },
                )"#]],
        );
    }

    #[test]
    fn parsing_smoke() {
        use crate::posting::platform::tests::assert_parse_query as test;
        test(
            "https://deviantart.com/miltvain/art/Twilight-magic-418078970",
            expect![
                r#"
                deviantart.com:DeviantArt(
                    Full {
                        author: "miltvain",
                        art: "Twilight-magic",
                        id: DeviationNumericId(
                            418078970,
                        ),
                    },
                )"#
            ],
        );
        test(
            "https://www.deviantart.com/miltvain/art/Twilight-magic-418078970",
            expect![[r#"
                www.deviantart.com:DeviantArt(
                    Full {
                        author: "miltvain",
                        art: "Twilight-magic",
                        id: DeviationNumericId(
                            418078970,
                        ),
                    },
                )"#]],
        );
        test(
            "https://deviantart.com/art/Twilight-magic-418078970",
            expect![[r#"
                deviantart.com:DeviantArt(
                    ArtAndId {
                        art: "Twilight-magic",
                        id: DeviationNumericId(
                            418078970,
                        ),
                    },
                )"#]],
        );
        test(
            "https://www.deviantart.com/art/Twilight-magic-418078970",
            expect![[r#"
                www.deviantart.com:DeviantArt(
                    ArtAndId {
                        art: "Twilight-magic",
                        id: DeviationNumericId(
                            418078970,
                        ),
                    },
                )"#]],
        );
        test(
            "https://deviantart.com/deviation/418078970",
            expect!["deviantart.com/deviation:DeviantArt(Id(DeviationNumericId(418078970)))"],
        );
        test(
            "https://wwww.deviantart.com/deviation/947204791",
            expect!["deviantart.com/deviation:DeviantArt(Id(DeviationNumericId(947204791)))"],
        );
        test(
            "https://view.deviantart.com/418078970",
            expect!["view.deviantart.com:DeviantArt(Id(DeviationNumericId(418078970)))"],
        );
    }
}
