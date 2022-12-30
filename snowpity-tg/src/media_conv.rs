use crate::prelude::*;
use crate::IoError;
use crate::{err_ctx, err, Result};
use std::process::Stdio;
use url::Url;

#[instrument]
// Some day we will use ffmpeg
#[allow(dead_code)]
pub(crate) async fn convert_to_mp4(input: &Url) -> Result<tempfile::TempPath> {
    let output = tempfile::NamedTempFile::new()
        .map_err(err_ctx!(IoError::CreateTempFile))?
        .into_temp_path();

    debug!(output = %output.display(), "Converting to mp4");

    let status = ffmpeg()
        .args([
            // Overwrite output file without interactive confirmation
            "-y",
            "-i",
            input.as_str(),
            // Set video bitrate
            "-b:v",
            "2000k",
            // Force input format
            "-f",
            "mp4",
        ])
        .arg(&output)
        .stdin(Stdio::null())
        .kill_on_drop(true)
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

fn ffmpeg() -> tokio::process::Command {
    tokio::process::Command::new("ffmpeg")
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum MediaConvError {
    #[error("Failed to spawn `ffmpeg` process")]
    SpawnFfmpeg { source: std::io::Error },

    #[error("Failed to wait for `ffmpeg` process")]
    WaitForFfmpeg { source: std::io::Error },

    #[error("`ffmpeg` failed with the exit code {status}")]
    Ffmpeg { status: std::process::ExitStatus },
}
