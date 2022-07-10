use crate::cmd;
use crate::util::{docker_compose_cmd, fs, repo_abs_path};
use clap::Parser;

/// Run the development instance of the bot using `docker compose`
#[derive(Parser, Debug)]
pub struct Start {
    #[clap(flatten)]
    build: cmd::Build,
}

impl cmd::Cmd for Start {
    fn run(self) -> anyhow::Result<()> {
        self.build.run()?;

        fs::create_dir_all(repo_abs_path(["data", "postgres"]))?;

        docker_compose_cmd()?.arg("up").run()?;

        Ok(())
    }
}
