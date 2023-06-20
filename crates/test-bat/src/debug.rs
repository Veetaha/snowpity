use std::fmt;
use super::{SnapshotFormat, Style};
use expect_test::Expect;

struct Imp;

impl<T: fmt::Debug> SnapshotFormat<T> for Imp {
    fn make_snapshot_imp(style: Style, actual: &T) -> String {
        match style {
            Style::Terse => format!("{actual:?}"),
            Style::Verbose => format!("{actual:#?}"),
        }
    }
}

/// Asserts that the [`fmt::Debug`] representation of `actual` is equal to the
/// given expected snapshot. Uses [`make_snapshot`] to make the
/// snapshot fit into a common width of a single screen.
#[track_caller]
pub fn assert_eq<T: fmt::Debug>(actual: T, expected: &Expect) {
    Imp::assert_eq(&actual, expected)
}

/// Same as [`assert_eq`], but specialized for [`Result`].
/// If the result is an [`Err`], then the snapshot will be prefixed
/// with `Err:` and the error will be formatted using [`make_snapshot`].
#[track_caller]
pub fn assert_result_eq<T, E>(actual: &Result<T, E>, expected: &Expect)
where
    T: fmt::Debug,
    E: std::error::Error,
{
    Imp::assert_result_eq(actual, expected)
}

/// Formats `actual` to string using [`fmt::Debug`] implementation of `actual`.
/// If its string length exceeds approximately a single-screen amount of characters,
/// it will be pretty-formatted with the `#` formatting specifier to fit its width
/// into a single screen.
pub fn make_snapshot<T: fmt::Debug>(actual: T) -> String {
    Imp::make_snapshot(&actual)
}
