mod gif_to_mp4;
mod webm_to_mp4;

pub(crate) use gif_to_mp4::*;
pub(crate) use webm_to_mp4::*;

use crate::Result;

// Rustfmt is doing a bad job of condensing this code, so let's disable it
#[rustfmt::skip]
const COMMON_ARGS: &[&str] = &[
    // Overwrite output file without interactive confirmation
    "-y",

    // Preserve the original FPS
    "-fps_mode",
    "passthrough",

    // MP4 videos using H.264 need to have a dimensions that are divisible by 2.
    // This option ensures that's the case.
    "-vf",
    "crop=floor(iw/2)*2:floor(ih/2)*2:0:0",

    "-c:v",
    "libx264",

    // Experimentally determined it to be the most optimal one for our server class
    "-preset",
    "faster",

    // Some video players require this setting, but Telegram doesn't seem to need
    // this. So let's not enable it and see where this gets us
    "-pix_fmt",
    "yuv420p",

    // It's the default value, but it's better to be explicit
    "-crf",
    "23",

    // Fast start is needed to make the video playable before it's fully downloaded
    "-movflags",
    "+faststart",
];

async fn ffmpeg(args: &[&str]) -> Result<Vec<u8>> {
    crate::util::process::run("ffmpeg", args).await
}
