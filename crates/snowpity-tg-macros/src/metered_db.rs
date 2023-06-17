use crate::Result;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

pub(crate) fn generate(_: TokenStream, item: TokenStream) -> Result<TokenStream2> {
    let func: syn::ItemFn = syn::parse(item)?;
    Ok(quote! {
        #[::metrics_bat::metered(metric = "crate::db::db_query_duration_seconds")]
        #func
    })
}
