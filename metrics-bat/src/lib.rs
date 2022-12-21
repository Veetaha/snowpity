//! Missing batteries for [`metrics`] crate.
//!
//! It includes various utilities to make defining labels and metrics with their
//! descriptions very laconic.
//!
//! ```
//! use metrics_bat::{labels, counters, gauges, histograms};
//!
//! labels! {
//!     HttpRequestLabels { method, status }
//!     ConnectedClientLabels { role }
//! }
//!
//! counters! {
//!     /// Metrics HELP message is required and goes in the doc comments
//!     http_requests_total;
//! }
//!
//! gauges! {
//!     /// Some HELP documentation here
//!     app_connected_clients_total;
//! }
//!
//! histograms! {
//!     /// The buckets are specified after the `=` sign:
//!     http_request_duration_seconds = [0.05, 0.1, 0.25, 0.5, 1.0, 2.5];
//! }
//!
//! // then we may use these the following way:
//!
//! let labels = HttpRequestLabels {
//!     // Anything that implements Into<metrics::SharedString> can be set as a value
//!     method: "GET",
//!     status: 200.to_string(),
//! };
//!
//! http_requests_total(labels.clone()).increment(1);
//! http_request_duration_seconds(labels).record(0.2);
//!
//! let labels = ConnectedClientLabels {
//!     role: "Admin"
//! };
//!
//! app_connected_clients_total(labels).set(99.0);
//! ```
//!
//! Then its possible to collect the buckets as configured by all [`histograms!`]
//! compiled in to the executable with [`default_histogram_buckets`]. Pay attention
//! that this is called "default" histogram buckets, because the users may want to
//! override the bucket spacing choice made by [`histograms!`], and they may
//! do this by filtering the histograms returned from [`default_histogram_buckets`].
//!
//! ```
//! use metrics_bat::prelude::*;
//!
//! metrics_exporter_prometheus::PrometheusBuilder::new()
//!     .set_default_buckets()
//!     .install()
//!     .expect("BUG: failed to initialize metrics");
//! ```

#[cfg(feature = "exporter-prometheus")]
mod exporter_prometheus;

mod timing;

pub use timing::HistogramTimer;

pub mod prelude {
    #[cfg(feature = "exporter-prometheus")]
    pub use crate::exporter_prometheus::PrometheusBuilderExt as _;
    pub use crate::timing::FutureExt as _;
    pub use crate::timing::HistogramExt as _;
}

/// Returns an iterator over histogram metric names and their respective buckets
/// configured at the call site of all histograms registered via [`histograms!`].
///
/// It collects the histogram buckets from any crate compiled into the executable,
/// so even if some 3-rd party library uses [`histograms!`], we are still able
/// to include its metrics in the returned iterator.
///
/// This function should be used with the metrics exporter to set the default
/// buckets as configured at the call site of [`histograms!`], if those
/// buckets are reasonable. You may filter any histogram metrics from the iterator
/// that you would like to override and specify buckets for them manually.
///
/// Acknowledgment: thanks David Tolnay for the awesome [`inventory`] crate!
pub fn default_histogram_buckets() -> impl Iterator<Item = (&'static str, &'static [f64])> {
    inventory::iter::<imp::Bucket>
        .into_iter()
        .map(|bucket| (bucket.metric, bucket.buckets))
}

/// Defines a struct with fields equal to label names, and values of generic
/// types, that each individually must implement [`Into`] [`metrics::SharedString`]
/// for the struct to implement [`metrics::IntoLabels`].
///
/// See crate-level doc for an example of usage.
#[macro_export]
macro_rules! labels {
    (
        $(
            $vis:vis $Labels:ident {
                $( $label:ident ),* $(,)?
            }
        )*
    ) => {
        $(
            // XXX: not qualifying the derives from `imp` because rust-analyzer
            // stops resolving the derived impls when we do this for some reason
            #[derive(Clone, Copy, Debug)]
            #[allow(non_camel_case_types)]
            $vis struct $Labels<$($label = $crate::imp::String,)*> {
                $( $vis $label: $label, )*
            }

            #[allow(non_camel_case_types)]
            impl<$($label,)*> $crate::imp::metrics::IntoLabels for $Labels<$($label,)*>
            where
                $($label: $crate::imp::Into<$crate::imp::metrics::SharedString>,)*
            {
                $vis fn into_labels(self) -> $crate::imp::Vec<$crate::imp::metrics::Label> {
                    vec![$($crate::imp::metrics::Label::new(stringify!($label), self.$label)),*]
                }
            }
        )*
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! metric_macros {
    (
        $macro_prefixed:ident,
        $macro_nonprefixed:ident,
        $($args:tt)*
    ) => {
        $crate::metric_macros!(@imp prefixed    $macro_prefixed    $($args)*);
        $crate::metric_macros!(@imp nonprefixed $macro_nonprefixed $($args)*);
    };
    (
        @imp
        $prefixed:tt
        $macro:ident
        $(#[doc = $macro_doc:literal])*
        $describe_macro:ident,
        $register_macro:ident,
        $metric_ty:ident,
        $d:tt
    ) => {

        #[doc = concat!(
            "Defines a function that accepts a value implementing [`metrics::IntoLabels`] and returns a ",
            "[`metrics::", stringify!($metric_ty), "`]\n\n",
            "This is the version of the macro that generates a metric name that is ",
            stringify!($prefixed), " with the name of the crate.\n\n",
            "See crate-level doc for an example of usage"
        )]
        $(#[doc = $macro_doc])*
        #[macro_export]
        macro_rules! $macro {
            ($d (
                $d ( #[doc = $d help:literal] )*
                $d vis:vis $d metric:ident $d ( = $d buckets:expr)?;
            )*) => {
                $d (
                    $d( #[doc = $d help] )*
                    $d vis fn $d metric(
                        labels: impl $crate::imp::metrics::IntoLabels
                    ) -> $crate::imp::metrics::$metric_ty
                    {
                        use $crate::imp::{std::sync::Once, metrics};

                        const METRIC: &str = $crate::metric_macros!(
                            @metric_name $prefixed $d metric
                        );

                        $crate::metric_macros!(@register_histogram METRIC, $d ($d buckets)?);

                        static DESCRIBE: Once = Once::new();
                        DESCRIBE.call_once(|| {
                            let help = [$d ($d help.trim()),*].join("\n");

                            metrics::$describe_macro!(METRIC, help);
                        });

                        metrics::$register_macro!(METRIC, labels)
                    }
                )*
            }
        }
    };
    (@metric_name prefixed $ident:ident) => {
        concat!(env!("CARGO_CRATE_NAME"), "_", stringify!($ident))
    };
    (@metric_name nonprefixed $ident:ident) => {
        stringify!($ident)
    };
    (@register_histogram $metric:expr, $buckets:expr) => {
        $crate::imp::inventory::submit! {
            $crate::imp::bucket($metric, &$buckets)
        }
    };
    (@register_histogram $metric:expr,) => {}
}

metric_macros! {
    counters,
    counters_nonprefixed,
    describe_counter,
    register_counter,
    Counter,
    $
}

metric_macros! {
    gauges,
    gauges_nonprefixed,
    describe_gauge,
    register_gauge,
    Gauge,
    $
}

metric_macros! {
    histograms,
    histograms_nonprefixed,
    describe_histogram,
    register_histogram,
    Histogram,
    $
}

#[doc(hidden)]
pub mod imp {
    pub use inventory;
    pub use metrics;
    pub use std;
    pub use std::prelude::rust_2021::*;

    pub struct Bucket {
        pub metric: &'static str,
        pub buckets: &'static [f64],
    }

    /// Use a function to get autoderef, so that we don't have to use `&` at
    /// the call site explicitly
    pub const fn bucket(metric: &'static str, buckets: &'static [f64]) -> Bucket {
        Bucket { metric, buckets }
    }

    inventory::collect!(Bucket);
}
