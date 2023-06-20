pub mod debug;
pub mod json;

use expect_test::Expect;

/// Approximate number of characters that can fit on a single screen
const COMMON_SCREEN_CHARS_WIDTH: usize = 60;

enum Style {
    Terse,
    Verbose,
}

trait SnapshotFormat<T> {
    fn make_snapshot_imp(style: Style, actual: &T) -> String;

    fn make_snapshot(actual: &T) -> String {
        let terse = Self::make_snapshot_imp(Style::Terse, actual);

        let Some(width) = terse.lines().map(|line| line.len()).max() else {
            return terse;
        };

        if width < COMMON_SCREEN_CHARS_WIDTH {
            return terse;
        }

        Self::make_snapshot_imp(Style::Verbose, actual)
    }

    #[track_caller]
    fn assert_eq(actual: &T, expected: &Expect) {
        expected.assert_eq(&Self::make_snapshot(actual))
    }

    #[track_caller]
    fn assert_result_eq<E: std::error::Error>(actual: &Result<T, E>, expected: &Expect) {
        let err = match actual {
            Ok(value) => return Self::assert_eq(value, expected),
            Err(err) => err,
        };

        let err = debug::make_snapshot(err);
        expected.assert_eq(&format!("Err:{err}"));
    }
}
