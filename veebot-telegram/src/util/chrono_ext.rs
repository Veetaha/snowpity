use chrono::prelude::*;
use easy_ext::ext;

#[ext(DateTimeExt)]
pub(crate) impl<Tz: chrono::TimeZone> DateTime<Tz> {
    fn to_ymd_hms(self) -> String
    where
        Tz::Offset: std::fmt::Display,
    {
        self.format("%Y-%m-%d %H:%M:%S").to_string()
    }
}

// pub(crate) fn _time_ago_from_now(past_date_time: DateTime<Utc>) -> String {
//     markdown::escape(&timeago::Formatter::new().convert_chrono(past_date_time, Utc::now()))
// }
