/// Convenience macro to define a [`prometheus`] metric.
///
/// The syntax is similar as of delcaring a variable with the type of the metric
/// as the type annotation. However, there is no `let` or `static` keyword required.
/// The optional initializer `= { .. }` accepts a mapping of `label_name => "label_value"`
/// pairs that will be used as constant labels for the metric.
///
/// The doc comments on top of the metric declaration will be used as the help text.
///
/// The resulting generated code contains a function with the name equal to the name
/// of the metricc. The function returns a static reference to the metric.
macro_rules! def_metrics {
    (
        $(
            $( #[doc = $help:literal] )*
            $vis:vis $name:ident: $ty:ident $(= {
                $($label_name:ident => $label_val:expr),* $(,)?
            })?;
        )*
    ) => {
        $(
            $( #[doc = $help] )*
            $vis fn $name() -> &'static ::prometheus::$ty {
                use ::once_cell::sync::OnceCell;
                use ::itertools::Itertools;

                static METRIC: OnceCell<::prometheus::$ty> = OnceCell::new();
                METRIC.get_or_init(|| {
                    let name = stringify!($name);
                    let help = [$($help),*]
                        .into_iter()
                        .map(|part| part.trim())
                        .join("\n");

                    let opts = ::prometheus::Opts::new(name, help)
                        .const_labels(<_>::from_iter([
                            $(
                                $((
                                    stringify!(label_name).to_owned(),
                                    $label_val.into()
                                )),*
                            )?
                        ]));

                    let metric = ::prometheus::$ty::with_opts(opts).unwrap();

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
