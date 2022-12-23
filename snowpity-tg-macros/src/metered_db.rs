use proc_macro2::TokenStream as TokenStream2;
use crate::Result;
use quote::quote;

pub(crate) fn generate(_: syn::AttributeArgs, func: syn::ItemFn) -> Result<TokenStream2> {
    Ok(quote! {
        #[::metrics_bat::metered(metric = "crate::db::db_query_duration_seconds")]
        #func
    })
}
