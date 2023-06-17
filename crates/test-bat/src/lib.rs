use expect_test::Expect;
use std::fmt;

/// Approximate number of characters that can fit on a single screen
const COMMON_SCREEN_CHARS_WIDTH: usize = 60;

/// Asserts that the debug representation of `actual` is equal to the
/// given expected snapshot. Uses [`make_debug_snapshot`] to make the
/// snapshot fit into a common width of a single screen.
#[track_caller]
pub fn assert_debug_eq(actual: &dyn fmt::Debug, expected: &Expect) {
    let mut actual_snapshot = format!("{actual:?}");

    if actual_snapshot.len() >= COMMON_SCREEN_CHARS_WIDTH {
        actual_snapshot = format!("{actual:#?}");
    }

    expected.assert_eq(&actual_snapshot)
}

/// Formats `actual` to string using [`fmt::Debug`] implementation of `actual`.
/// If its string length exceeds approximately a single-screen amount of characters,
/// it will be pretty-formatted with the `#` formatting specifier to fit its width
/// into a single screen.
pub fn make_debug_snapshot(actual: &dyn fmt::Debug) -> String {
    let terse = format!("{actual:?}");

    let Some(width) = terse.lines().map(|line| line.len()).max() else {
        return terse;
    };

    if width >= COMMON_SCREEN_CHARS_WIDTH {
        return format!("{actual:#?}");
    }

    terse
}
