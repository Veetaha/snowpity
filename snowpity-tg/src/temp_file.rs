use crate::prelude::*;
use crate::Result;

pub(crate) async fn create_temp_file() -> Result<tempfile::NamedTempFile> {
    tokio::task::spawn_blocking(|| {
        tempfile::NamedTempFile::new().fatal_ctx(|| "Failed to create a temporary file")
    })
    .await
    .unwrap()
}
