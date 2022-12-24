use std::time::Duration;

use chrono::prelude::*;
use easy_ext::ext;

#[ext(DateTimeExt)]
pub(crate) impl<Tz: chrono::TimeZone> DateTime<Tz> {
    fn to_human_readable(self) -> String
    where
        Tz::Offset: std::fmt::Display,
    {
        // Uses the timezone where @Veetaha lives for their convenience :D
        self.with_timezone(&FixedOffset::east(2 * 60 * 60))
            .format("%Y-%m-%d %H:%M:%S (GMT%:z)")
            .to_string()
    }
}

// pub(crate) fn _time_ago_from_now(past_date_time: DateTime<Utc>) -> String {
//     markdown::escape(&timeago::Formatter::new().convert_chrono(past_date_time, Utc::now()))
// }

pub(crate) fn human_duration(duration: Duration) -> String {
    timeago::Formatter::new()
        .num_items(3)
        .ago("")
        .convert(duration)
}
