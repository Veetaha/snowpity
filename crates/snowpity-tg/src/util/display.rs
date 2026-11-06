pub(crate) fn human_size(bytes: impl humansize::ToF64 + humansize::Unsigned) -> String {
    humansize::format_size(bytes, humansize::DECIMAL)
}
