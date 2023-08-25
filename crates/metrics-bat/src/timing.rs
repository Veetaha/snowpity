use easy_ext::ext;
use metrics::{Histogram, Label};
use std::future::Future;
use std::pin::Pin;
use std::task::Poll;
use std::time::Instant;

const RESULT_LABEL: &str = "result";

#[ext(HistogramExt)]
pub impl Histogram {
    fn start_timer(self) -> HistogramTimer {
        HistogramTimer {
            start: Instant::now(),
            histogram: self,
        }
    }
}

// The following code was copied and adapted from:
// https://github.com/tikv/rust-prometheus/blob/8b462b194565d35e84def2d27ca8efd4d395a7c9/src/histogram.rs#L574-L650

/// Timer to measure and record the duration of an event.
///
/// This timer can be stopped and recorded at most once, either automatically (when it
/// goes out of scope) or manually.
#[must_use = "Timer should be kept in a variable otherwise it cannot record duration"]
pub struct HistogramTimer {
    /// A histogram for automatic recording of observations.
    histogram: Histogram,
    /// Starting instant for the timer.
    start: Instant,
}

impl HistogramTimer {
    /// Observe and record timer duration (in seconds).
    ///
    /// It observes the floating-point number of seconds elapsed since the timer
    /// started, and it records that value to the attached histogram.
    pub fn record_duration(self) {}
}

impl Drop for HistogramTimer {
    fn drop(&mut self) {
        self.histogram.record(self.start.elapsed());
    }
}

#[ext(FutureExt)]
pub impl<F: Future + Sized> F {
    fn record_duration<Fn, L>(
        self,
        make_histogram: Fn,
        labels: L,
    ) -> RecordDurationFuture<Self, Fn, L> {
        RecordDurationFuture {
            future: self,
            record: Some((make_histogram, labels, Instant::now())),
        }
    }
}

pin_project_lite::pin_project! {
    pub struct RecordDurationFuture<F, Fn, L> {
        #[pin]
        future: F,
        record: Option<(Fn, L, Instant)>,
    }
}

impl<F, Ok, Err, Fn, L> Future for RecordDurationFuture<F, Fn, L>
where
    F: Future<Output = Result<Ok, Err>>,
    Fn: FnOnce(Vec<metrics::Label>) -> Histogram,
    L: metrics::IntoLabels,
{
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let output = std::task::ready!(this.future.poll(cx));

        let Some((make_histogram, labels, start)) = this.record.take() else {
            return Poll::Ready(output);
        };

        let mut labels = labels.into_labels();

        let existing_result = labels.iter().find(|label| label.key() == RESULT_LABEL);

        let Some(existing_result) = existing_result else {
            let result = match output {
                Ok(_) => "ok",
                Err(_) => "err",
            };
            labels.push(Label::from_static_parts(RESULT_LABEL, result));
            make_histogram(labels).record(start.elapsed());
            return Poll::Ready(output);
        };

        let message = format!(
            "BUG: label uses a name `{RESULT_LABEL}` that is reserved for \
            recording the result of a future: {existing_result:?}"
        );

        debug_assert!(false, "{message}");
        tracing::error!("{message}");

        Poll::Ready(output)
    }
}
