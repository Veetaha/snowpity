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
