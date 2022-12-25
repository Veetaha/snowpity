use crate::prelude::*;
use crate::IoError;
use crate::{err_ctx, err_val, MediaError, Result};
use std::process::Stdio;
use url::Url;

#[instrument]
pub(crate) async fn convert_to_mp4(input: &Url) -> Result<tempfile::TempPath> {
    let output = tempfile::NamedTempFile::new()
        .map_err(err_ctx!(IoError::CreateTempFile))?
        .into_temp_path();

    debug!(output = %output.display(), "Converting to mp4");

    let status = ffmpeg()
        .args(["-i", input.as_str(), "-b:v", "2000k", "-f", "mp4", "-y"])
        .arg(&output)
        .stdin(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(err_ctx!(MediaError::SpawnFfmpeg))?
        .wait()
        .await
        .map_err(err_ctx!(MediaError::WaitForFfmpeg))?;

    if status.success() {
        return Ok(output);
    }

    Err(err_val!(MediaError::Ffmpeg { status }))
}

fn ffmpeg() -> tokio::process::Command {
    tokio::process::Command::new("ffmpeg")
}
