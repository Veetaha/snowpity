/// Convenience macro to define a [`prometheus`] metric.
///
/// The syntax is similar as of declaring a variable with the type of the metric
/// as the type annotation. However, there is no `let` or `static` keyword required.
/// The optional block `{ .. }` accepts a mapping of `const_label => "const_label_val"`
/// pairs that will be used as constant labels for the metric. Another optional
/// block `[ .. ]` accepts the list of identifiers for the variable labels.
///
/// The doc comments on top of the metric declaration will be used as the help text.
///
/// The resulting generated code contains a function with the name equal to the name
/// of the metricc. The function returns a static reference to the metric.
macro_rules! def_metrics {
    (@construct $opts:ident, $ty:ident, ) => {
        ::prometheus::$ty::with_opts($opts)
    };
    (@construct $opts:ident, $ty:ident, [$($label:ident),*]) => {
        ::prometheus::$ty::new($opts, &[$(stringify!($label)),*])
    };
    (@opts $name:expr, $help:expr) => {
        ::prometheus::Opts::new($name, $help)
    };
    (@opts $name:expr, $help:expr, $buckets:expr) => {
        ::prometheus::HistogramOpts::new($name, $help).buckets(Vec::from_iter($buckets))
    };
    (
        $(
            $( #[doc = $help:literal] )*
            $( [buckets: $buckets:expr] )?
            $name:ident: $ty:ident

            $({ $($const_label:ident => $const_label_val:expr),* $(,)? })?

            $([ $($variable_label:ident),* $(,)? ])?;
        )*
    ) => {
        $(
            mod $name {
                // The name of the constant relects the label name 1-to-1,
                // this is neat, and not following a SCREAMING_SNAKE_CASE
                // isn't a problem due to this.
                $($(
                    #[allow(non_upper_case_globals)]
                    pub(super) const $variable_label: &'static str = stringify!($variable_label);
                )*)?
            }

            $( #[doc = $help] )*
            fn $name() -> &'static ::prometheus::$ty {
                use ::once_cell::sync::OnceCell;
                use ::itertools::Itertools;

                static METRIC: OnceCell<::prometheus::$ty> = OnceCell::new();
                METRIC.get_or_init(|| {
                    let name = stringify!($name);
                    let help = [$($help),*]
                        .into_iter()
                        .map(|part| part.trim())
                        .join("\n");

                    let opts = $crate::metrics::def_metrics!(@opts name, help $(, $buckets)?)
                        .const_labels(<_>::from_iter([
                            $(
                                $((
                                    stringify!(const_label).to_owned(),
                                    $const_label_val.into()
                                )),*
                            )?
                        ]));

                    let metric = $crate::metrics::def_metrics!(
                        @construct opts, $ty, $([$($variable_label),*])?
                    )
                    .unwrap();

                    ::prometheus::register(Box::new(metric.clone()))
                        .unwrap_or_else(|err| {
                            panic!("BUG: failed to register `{name}` metric: {err:#?}")
                        });

                    metric
                })
            }
        )*
    };
}

pub(crate) use def_metrics;
