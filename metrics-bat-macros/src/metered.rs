use crate::Result;
use darling::FromMeta;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

#[derive(FromMeta)]
struct TestOpts {
    /// Path to the metric producer function defined with `metrics_bat::historgams!` macro.
    metric: syn::Path,

    /// Optional list of labels to add to the metric. If omitted, no other
    /// labels than the default one will be added. Note that it doesn't cancel
    /// default labels from adding, it only extends them.
    ///
    /// More on default labels in [`crate::metered`].
    labels: Option<syn::Expr>,
}

pub(crate) fn generate(opts: syn::AttributeArgs, mut func: syn::ItemFn) -> Result<TokenStream2> {
    let opts = TestOpts::from_list(&opts)?;

    let home_crate = quote!(::metrics_bat);
    let imp = quote!(#home_crate::imp::proc_macros);
    let metrics = quote!(#imp::metrics);

    if func.sig.asyncness.is_none() {
        return Err(darling::Error::custom("the metered function must be async")
            .with_span(&func.sig.fn_token));
    }

    if !is_result(&func.sig.output) {
        return Err(
            darling::Error::custom("the metered function must return a Result")
                .with_span(&func.sig.output),
        );
    }

    let fn_block = func.block;
    let metric = opts.metric;

    let span_label = quote! {
        #metrics::Label::from_static_parts(
            "path",
            #imp::type_name_of_val(&|| {}).trim_end_matches("::{{closure}}")
        )
    };

    let labels = match opts.labels {
        Some(labels) => quote! {{
            let mut labels = #labels;
            labels.push(#span_label);
            labels
        }},
        None => quote!(vec![#span_label]),
    };

    func.block = syn::parse_quote!({
        let __labels = #labels;
        #imp::FutureExt::record_duration(async move #fn_block, #metric, __labels).await
    });

    Ok(quote!(#func))
}

fn is_result(return_type: &syn::ReturnType) -> bool {
    let ty = match return_type {
        syn::ReturnType::Default => return false,
        syn::ReturnType::Type(_, ty) => ty,
    };

    let syn::Type::Path(ty) = discard_type_group(ty) else {
        return false
    };

    ty.path
        .segments
        .last()
        .expect("BUG: the path must consist of at least one segment")
        .ident
        .to_string()
        .ends_with("Result")
}

fn discard_type_group(ty: &syn::Type) -> &syn::Type {
    match ty {
        syn::Type::Group(ty) => discard_type_group(&ty.elem),
        _ => ty,
    }
}
