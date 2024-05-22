use super::{ffmpeg, COMMON_ARGS};
use crate::prelude::*;
use crate::Result;
use std::path::Path;

#[instrument(skip_all, fields(input = %input.as_ref().display()))]
pub(crate) async fn gif_to_mp4(input: impl AsRef<Path>) -> Result<tempfile::TempPath> {
    let input = input.as_ref();

    let output = std::env::temp_dir().join(format!("{}.mp4", nanoid::nanoid!()));
    let log_message = format!("Converting GIF to mp4 with output at {output:?}");

    let output = tempfile::TempPath::from_path(output);

    // This is inspired a bit by this code:
    // https://github.com/philomena-dev/philomena/blob/master/lib/philomena/processors/gif.ex#L96

    let input_arg = input.to_string_lossy();
    let output_arg = output.to_string_lossy();

    // Rustfmt is doing a bad job of condensing this code, so let's disable it
    #[rustfmt::skip]
    let args = [
        &[
            // Force GIF format of the input
            "-f",
            "gif",

            // Set input path
            "-i",
            &input_arg,
        ],
        COMMON_ARGS,
        &[
            // No audio channel is needed at all, because GIFs don't have sound
            "-an",
            &output_arg,
        ],
    ]
    .concat();

    ffmpeg(&args).with_duration_log(&log_message).await?;

    Ok(output)
}
