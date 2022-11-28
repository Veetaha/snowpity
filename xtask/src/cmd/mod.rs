mod fmt_aliases;

pub(crate) use fmt_aliases::*;

pub(crate) trait Cmd {
    fn run(self) -> anyhow::Result<()>;
}
