use crate::util::{docker_compose_cmd};
use clap::Parser;

/// Build the image to run with `start` command.
#[derive(Parser, Debug)]
pub struct Build {
    /// Build in release mode with optimizations.
    /// The build happens in debug mode by default.
    #[clap(long)]
    release: bool,
}

impl crate::cmd::Cmd for Build {
    fn run(self) -> anyhow::Result<()> {
        let mut cmd = docker_compose_cmd()?;

        cmd.arg("build");

        if self.release {
            cmd.arg2("--build-arg", "release");
        }

        cmd.run()?;

        Ok(())
    }
}
