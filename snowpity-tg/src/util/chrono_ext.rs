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
