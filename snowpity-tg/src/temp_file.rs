use crate::prelude::*;
use crate::Result;
use easy_ext::ext;
use tempfile::NamedTempFile;

pub(crate) async fn create_temp_file() -> Result<NamedTempFile> {
    tokio::task::spawn_blocking(move || {
        tempfile::Builder::new()
            .tempfile()
            .fatal_ctx(|| format!("Failed to create a temporary file"))
    })
    .await
    .unwrap()
}

#[ext(NamedTempFileExt)]
pub(crate) impl NamedTempFile {
    fn into_tokio(self) -> (tokio::fs::File, tempfile::TempPath) {
        let (file, path) = self.into_parts();
        (tokio::fs::File::from_std(file), path)
    }
}
