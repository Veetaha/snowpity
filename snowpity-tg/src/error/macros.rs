/// Macro to reduce the boilerplate of creating crate-level errors.
/// It directly accepts the body of [`ErrorKind`] variant without type name qualification.
/// It also automatically calls [`Into`] conversion for each passed field.
macro_rules! err {
    (@val $variant_ident:ident $field_val:expr) => ($field_val);
    (@val $variant_ident:ident) => ($variant_ident);
    ($variant_path:path $({
        $( $field_ident:ident $(: $field_val:expr)? ),*
        $(,)?
    })?) => {{
        use $variant_path as Variant;

        $crate::error::Error::from(
            Variant $({$(
                $field_ident: ::std::convert::Into::into(
                    $crate::error::err!(@val $field_ident $($field_val)?)
                )
            ),*})?
        )
    }};
}

/// Shortcut for defining `map_err` closures that automatically forwards `source`
/// error to the variant.
macro_rules! err_ctx {
    ($variant_path:path $({ $($variant_fields:tt)* })?) => {
        |source| $crate::error::err!($variant_path { source, $($($variant_fields)*)? })
    };
}

/// Creates a [`ErrorKind::Fatal`] error with the given formatting string
macro_rules! fatal {
    ($($arg:tt)*) => {
        $crate::error::err!($crate::ErrorKind::Fatal {
            message: format!($($arg)*),
            source: None,
        })
    };
}

pub(crate) use err;
pub(crate) use err_ctx;
pub(crate) use fatal;
