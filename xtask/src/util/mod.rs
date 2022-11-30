use std::path::{Path, PathBuf};

pub(crate) fn repo_abs_path<I>(components: I) -> PathBuf
where
    I: IntoIterator,
    I::Item: AsRef<Path>,
{
    let mut path = repo_root();
    path.extend(components);
    path
}

pub(crate) fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_owned()
}
