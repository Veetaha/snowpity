mod metered_db;

use proc_macro::TokenStream;

type Result<T = (), E = darling::Error> = std::result::Result<T, E>;

/// Shortcut for `#[metrics_bat::metered]` with db query duration metric.
#[proc_macro_attribute]
pub fn metered_db(opts: TokenStream, item: TokenStream) -> TokenStream {
    metered_db::generate(opts, item).map_or_else(|err| err.write_errors().into(), Into::into)
}
