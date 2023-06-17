use crate::prelude::*;
use crate::Result;
use std::path::Path;

// TODO: we need these dependencies to resize images to fit into telegram's limits
// Adding `use` statements to make sure `cargo-machete` doesn't mark them as unused
#[allow(clippy::single_component_path_imports, unused_imports)]
use fast_image_resize;
#[allow(clippy::single_component_path_imports, unused_imports)]
use image;

// pub(crate) async fn resize_image(data: bytes::Bytes) -> Result<tempfile::TempPath> {
//     // fast_image_resize::Image::from_slice_u8(width, height, buffer, fast_image_resize::PixelType::)
//     // use image::
//     // use fast_image_resize::
//     todo!()
// }

#[instrument]
pub(crate) async fn gif_to_mp4(input: &Path) -> Result<tempfile::TempPath> {
    let output = std::env::temp_dir().join(format!("{}.mp4", nanoid::nanoid!()));
    let log_message = format!("Converting GIF to mp4 with output at {output:?}");

    let output = tempfile::TempPath::from_path(output);

    // This is inspired a bit by this code:
    // https://github.com/philomena-dev/philomena/blob/master/lib/philomena/processors/gif.ex#L96

    // Rustfmt is doing a bad job of condensing this code, so let's disable it
    #[rustfmt::skip]
    ffmpeg(&[
        // Overwrite output file without interactive confirmation
        "-y",

        // Force GIF format of the input
        "-f",
        "gif",

        // Set input path
        "-i",
        &input.to_string_lossy(),

        // Preserve the original FPS
        "-fps_mode",
        "passthrough",

        // MP4 videos using H.264 need to have a dimensions that are divisible by 2.
        // This option ensures that's the case.
        "-vf",
        "scale=ceil(iw/2)*2:ceil(ih/2)*2",

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

        // No audio channel is needed at all, because GIFs don't have sound
        "-an",

        &output.to_string_lossy(),
    ])
    .with_duration_log(&log_message)
    .await?;

    Ok(output)
}

async fn ffmpeg(args: &[&str]) -> Result<Vec<u8>> {
    crate::util::process::run("ffmpeg", args).await
}
