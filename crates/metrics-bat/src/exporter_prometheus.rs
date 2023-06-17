use metrics_exporter_prometheus::{Matcher, PrometheusBuilder};

pub trait PrometheusBuilderExt {
    fn set_default_buckets(self) -> Self;
}

impl PrometheusBuilderExt for PrometheusBuilder {
    fn set_default_buckets(mut self) -> Self {
        for (histogram_name, buckets) in crate::default_histogram_buckets() {
            self = self
                .set_buckets_for_metric(Matcher::Full(histogram_name.to_owned()), buckets)
                .unwrap_or_else(|err| {
                    panic!(
                        "BUG: histogram metric `{histogram_name}` defined \
                        empty list of buckets: {err:?}"
                    )
                });
        }
        self
    }
}
