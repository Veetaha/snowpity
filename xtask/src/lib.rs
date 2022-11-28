mod cmd;
mod util;

use clap::Parser;
use cmd::Cmd;

/// Assorted development scripts for this repository that are easier
/// to write in Rust than using a shell scripting language.
#[derive(Parser, Debug)]
enum Args {
    FmtAliases(cmd::FmtAliases),
}

pub fn run() -> anyhow::Result<()> {
    match Args::parse() {
        Args::FmtAliases(cmd) => cmd.run(),
    }
}
