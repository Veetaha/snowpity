use super::{ffmpeg, COMMON_ARGS};
use crate::prelude::*;
use crate::Result;
use std::path::Path;
use tempfile::TempPath;

#[instrument(skip_all, fields(input = %input.as_ref().display()))]
pub(crate) async fn webm_to_mp4(input: impl AsRef<Path>) -> Result<TempPath> {
    let input = input.as_ref();

    let output = std::env::temp_dir().join(format!("{}.mp4", nanoid::nanoid!()));
    let log_message = format!("Converting Webm to mp4 with output at {output:?}");

    let output = tempfile::TempPath::from_path(output);

    // This is inspired a bit by this code:
    // https://github.com/philomena-dev/philomena/blob/master/lib/philomena/processors/gif.ex#L96

    let input_arg = input.to_string_lossy();
    let output_arg = output.to_string_lossy();

    let args = [
        &["-f", "webm", "-i", &input_arg],
        COMMON_ARGS,
        &[&output_arg],
    ]
    .concat();

    ffmpeg(&args).with_duration_log(&log_message).await?;

    Ok(output)
}
