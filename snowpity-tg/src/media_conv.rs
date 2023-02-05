use crate::prelude::*;
use crate::temp_file::create_temp_file;
use crate::{err, err_ctx, Result};
use std::process::Stdio;
use url::Url;

#[instrument]
pub(crate) async fn gif_to_mp4(input: &[u8]) -> Result<tempfile::TempPath> {
    let input = tempfile::NamedTempFile::new()
        .map_err(err_ctx!(IoError::CreateTempFile))?
        .into_temp_path();

    let output = tempfile::NamedTempFile::new()
        .map_err(err_ctx!(IoError::CreateTempFile))?
        .into_temp_path();

    debug!(output = %output.display(), "Converting to mp4");

    let status = ffmpeg([
            // Overwrite output file without interactive confirmation
            "-y",
            "-i",
            input.as_str(),
            // ,
            // MP4 videos using H.264 need to have a dimensions that are divisible by 2.
            // This option ensures that's the case.
            "scale=ceil(iw/2)*2:ceil(ih/2)*2",

            // Set output format
            // "-f",
            // "mp4",
        ])
        .arg(&output)
        .stdin(Stdio::null())
        .spawn()
        .map_err(err_ctx!(MediaConvError::SpawnFfmpeg))?
        .wait()
        .await
        .map_err(err_ctx!(MediaConvError::WaitForFfmpeg))?;

    if status.success() {
        return Ok(output);
    }

    Err(err!(MediaConvError::Ffmpeg { status }))
}

async fn ffmpeg(args: Vec<String>) -> Result<Vec<u8>> {
    debug!(
        cmd = %shlex::join(args.iter().map(String::as_str)),
        "Running ffmpeg"
    );

    let output = tokio::process::Command::new("ffmpeg")
        .args(args)
        .kill_on_drop(true)
        .output()
        .await?
        .stdout;

    Ok(output)
}
