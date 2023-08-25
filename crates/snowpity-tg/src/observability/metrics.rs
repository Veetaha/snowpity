use super::GLOBAL_LABELS;
use crate::util::units::{KB, MB};
use metrics_bat::prelude::*;

/// Histogram buckets to measure the distribution of request durations in seconds
pub(crate) const DEFAULT_DURATION_BUCKETS: &[f64] = &[
    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];

const KB_F: f64 = KB as f64;
const MB_F: f64 = MB as f64;

pub(crate) const DEFAULT_BLOB_SIZE_BUCKETS: &[f64] = &[
    KB_F * 4.,
    KB_F * 16.,
    KB_F * 64.,
    KB_F * 256.,
    MB_F * 1.,
    MB_F * 2.,
    MB_F * 4.,
    MB_F * 6.,
    MB_F * 8.,
    MB_F * 10.,
    MB_F * 20.,
    MB_F * 50.,
];

pub fn init_metrics() {
    let mut builder = metrics_exporter_prometheus::PrometheusBuilder::new()
        .with_http_listener(([0, 0, 0, 0], 2000))
        .set_default_buckets();

    for (key, value) in GLOBAL_LABELS {
        builder = builder.add_global_label(*key, *value);
    }

    builder
        .install()
        .expect("BUG: failed to initialize the metrics listener");
}
