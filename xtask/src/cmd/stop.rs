use crate::util::docker_compose_cmd;
use clap::Parser;

/// Stop the development instance of the bot using `docker compose`
#[derive(Parser, Debug)]
pub struct Stop {}

impl crate::cmd::Cmd for Stop {
    fn run(self) -> anyhow::Result<()> {
        docker_compose_cmd()?.arg("down").run()?;

        Ok(())
    }
}
