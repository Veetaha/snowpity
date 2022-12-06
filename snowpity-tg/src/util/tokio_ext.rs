/// Synchronously waits for the [`tokio::task::JoinHandle`].
///
/// Technically, this is a crime, but we don't have async drop in rust yet
/// and doing cleanup manually is a bigger crime IMHO. This function should
/// be used in [`Drop`] impls only.
///
/// Doesn't panic if the thread already is panicking (e.g in [`Drop`] impl).
pub(crate) fn block_in_place(join_handle: tokio::task::JoinHandle<()>) {
    let result =
        tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(join_handle));

    if let Err(err) = result {
        if std::thread::panicking() {
            eprintln!("JoinError waiting for task shutdown: {err:#?}");
        } else {
            panic!("JoinError waiting for task shutdown: {err:#?}");
        }
    }
}

/// Same as [`tokio::spawn_blocking`], but propagates panics in the spawned task
/// to the caller that will await the returned future.
pub(crate) async fn spawn_blocking<F, R>(f: F) -> R
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .unwrap_or_else(|err| panic!("Blocking task finished with an error: {err:#?}"))
}
