use crate::posting::{parse_with_regexes, ParseQueryResult, RequestId};

pub(crate) fn parse_query(input: &str) -> ParseQueryResult<'_, TweetId> {
    // The regex was inspired by the one in the booru/scraper repository:
    // https://github.com/booru/scraper/blob/095771b28521b49ae67e30db2764406a68b74395/src/scraper/twitter.rs#L16
    let (_, host, id) = parse_with_regexes!(
        input,
        r"((?:(?:mobile\.)|vx)?twitter.com)/[A-Za-z\d_]+/status/(\d+)",
    )?;

    Some((host, id.parse().ok()?))
}

#[cfg(test)]
mod tests {
    use expect_test::expect;

    #[test]
    fn smoke() {
        use crate::sites::tests::assert_parse_query as test;
        test(
            "https://twitter.com/NORDING34/status/1607191066318454791",
            expect!["twitter.com:Twitter(TweetId(1607191066318454791))"],
        );
        test(
            "https://vxtwitter.com/NORDING34/status/1607191066318454791",
            expect!["vxtwitter.com:Twitter(TweetId(1607191066318454791))"],
        );
        test(
            "https://mobile.twitter.com/NORDING34/status/1607191066318454791",
            expect!["mobile.twitter.com:Twitter(TweetId(1607191066318454791))"],
        );
    }
}
