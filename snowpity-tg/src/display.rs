use std::time::Duration;

pub(crate) fn human_size(bytes: impl humansize::ToF64 + humansize::Unsigned) -> String {
    humansize::format_size(bytes, humansize::BINARY)
}

pub(crate) fn human_duration(duration: Duration) -> String {
    timeago::Formatter::new()
        .num_items(3)
        .ago("")
        .convert(duration)
}
