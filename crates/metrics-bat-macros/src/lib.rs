mod metered;

use proc_macro::TokenStream;

type Result<T = (), E = darling::Error> = std::result::Result<T, E>;

/// Records the duration of the annotated function with the given metric.
///
/// It always adds the following default labels to the metric:
///
/// - `path`: the module path to the closest surrounding named function.
///   It is inferred using `type_name_of_val` and trimming `::{{closure}}` suffixes.
///
/// - `result`: present only when the function returns a `Result` type, and
///   is set to `ok` or `err` depending on the outcome of the function.
///
/// Note that the detection of the `Result` type is done by checking if the
/// return type name ends with `Result` suffix.
#[proc_macro_attribute]
pub fn metered(opts: TokenStream, item: TokenStream) -> TokenStream {
    metered::generate(opts, item).map_or_else(|err| err.write_errors().into(), Into::into)
}
