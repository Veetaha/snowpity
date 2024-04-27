use crate::posting::platform::prelude::*;
use crate::prelude::*;
use crate::Result;

use itertools::Either;
use reqwest::Url;
use serde::Deserialize;

use self::derpitools::DerpiPlatformKind;

mod api;
mod db;
mod derpitools;

pub(crate) mod derpibooru;
pub(crate) mod ponerpics;
pub(crate) mod twibooru;

#[derive(Clone, Deserialize)]
pub(crate) struct Config {
    // Derpilike platforms doesn't require an API key for read-only requests.
    // The rate limiting is also the same for both anonymous and authenticated requests,
    // therefore we don't really need an API key
    //
    // For Derpibooru, this was confirmed by the Derpibooru staff in discord:
    // https://discord.com/channels/430829008402251796/438029140659142657/1059492359122989146
    //
    // This config struct exists here, just in case some day we do need to use an API key,
    // or want any other config options.
    //
    // api_key: String,
}

impl ConfigTrait for Config {
    // TODO                           _________
    const ENV_PREFIX: &'static str = "DERPILIKE_";
}

impl DisplayInFileNameViaToString for api::MediaId {}

/// URL of the media that best suits Telegram.
///
/// Right now this is just the `view_url`, i.e. the original image representation.
/// Best would be if derpibooru could generate the representation of an image for
/// 2560x2560 pixels, but the biggest non-original representation is 1280x1024,
/// according to philomena's [sources].
///
/// This doesn't however guarantee the images will have top-notch quality (see [wiki]).
/// The GIFs don't use the `passthrough` flag when they are converted to MP4,
/// which means the FPS of the MP4 may be lower than the original GIF, so we
/// are re-generating the MP4 on the fly ourselves.
///
/// [wiki]: https://github.com/Veetaha/snowpity/wiki/Telegram-images-compression
/// [sources]: https://github.com/philomena-dev/philomena/blob/743699c6afe38b20b23f866c2c1a590c86d6095e/lib/philomena/images/thumbnailer.ex#L16-L24
fn best_tg_reprs(media: &api::Media, platform_kind: DerpiPlatformKind) -> Vec<(Url, BlobKind)> {
    match media.mime_type {
        api::MimeType::ImageJpeg => vec![(media.view_url.clone(), BlobKind::ImageJpeg)],
        api::MimeType::ImagePng => vec![(media.view_url.clone(), BlobKind::ImagePng)],
        api::MimeType::ImageSvgXml => vec![(media.view_url.clone(), BlobKind::ImageSvg)],
        api::MimeType::ImageGif => {
            if let DerpiPlatformKind::Twibooru = platform_kind {
                return vec![(media.view_url.clone(), BlobKind::AnimationGif)];
            }
            vec![
                // First of all try to get an existing MP4 representation for the GIF
                (media.unwrap_mp4_url(), BlobKind::AnimationMp4),
                // If there is no MP4 representation, then generate it on the fly
                // from the original GIF file
                (media.view_url.clone(), BlobKind::AnimationGif),
            ]
        }
        api::MimeType::VideoWebm => {
            if let DerpiPlatformKind::Twibooru = platform_kind {
                return vec![(media.view_url.clone(), BlobKind::VideoMp4)];
            }
            vec![(media.unwrap_mp4_url(), BlobKind::VideoMp4)]
        }
        api::MimeType::VideoMp4 => vec![(media.unwrap_mp4_url(), BlobKind::VideoMp4)],
    }
}

#[cfg(test)]
mod tests {
    use expect_test::expect;

    #[test]
    fn smoke() {
        use crate::posting::platform::tests::assert_parse_query as test;
        test(
            "derpibooru.org/123/",
            expect!["derpibooru.org:Derpibooru(MediaId(123))"],
        );
        test(
            "derpibooru.org/123",
            expect!["derpibooru.org:Derpibooru(MediaId(123))"],
        );
        test(
            "derpibooru.org/images/123",
            expect!["derpibooru.org/images:Derpibooru(MediaId(123))"],
        );
        test(
            "derpibooru.org/images/123/",
            expect!["derpibooru.org/images:Derpibooru(MediaId(123))"],
        );
        test(
            "https://derpicdn.net/img/2022/12/17/3008328/large.jpg",
            expect!["derpicdn.net/img:Derpibooru(MediaId(3008328))"],
        );
        test(
            "https://derpicdn.net/img/view/2022/12/17/3008328.jpg",
            expect!["derpicdn.net/img/view:Derpibooru(MediaId(3008328))"],
        );
        test(
            "https://derpicdn.net/img/download/2022/12/28/3015836__safe_artist-colon-shadowreindeer_foo.jpg",
            expect!["derpicdn.net/img/download:Derpibooru(MediaId(3015836))"]
        );
    }
}
