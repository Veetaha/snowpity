mod cmd;
mod util;

use clap::Parser;
use cmd::Cmd;

/// Assorted development scripts for this repository
#[derive(Parser, Debug)]
enum Args {
    Build(cmd::Build),
    Start(cmd::Start),
    Stop(cmd::Stop),
    CleanData(cmd::CleanData),
    FmtAliases(cmd::FmtAliases),
}

pub fn run() -> anyhow::Result<()> {
    match Args::parse() {
        Args::Build(cmd) => cmd.run(),
        Args::Start(cmd) => cmd.run(),
        Args::Stop(cmd) => cmd.run(),
        Args::CleanData(cmd) => cmd.run(),
        Args::FmtAliases(cmd) => cmd.run(),
    }
}
