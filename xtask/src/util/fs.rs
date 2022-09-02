pub use fs_err::*;
use std::io;
use std::path::Path;

pub(crate) fn remove_dir_all_if_exists(path: &Path) -> io::Result<()> {
    let result = fs_err::remove_dir_all(path);

    if is_not_found(&result) {
        return Ok(());
    }

    result
}

// pub(crate) fn metadata(path: &Path) -> io::Result<Option<std::fs::Metadata>> {
//     not_found_as_none(fs_err::metadata(path))
// }

// fn not_found_as_none<T>(result: io::Result<T>) -> io::Result<Option<T>> {
//     if is_not_found(&result) {
//         return Ok(None);
//     }
//     result.map(Some)
// }

fn is_not_found<T>(result: &io::Result<T>) -> bool {
    matches!(result, Err(err) if err.kind() == io::ErrorKind::NotFound)
}
