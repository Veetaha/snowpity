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
        // let pg_admin_dir = repo_abs_path(["data", "pgadmin"]);

        // if pg_admin_dir.exists() {


        //     fs::create_dir_all(pg_admin_dir);
        //     // std::os::unix::fs::chown(dir, uid, gid)


        //     std::sys::fs::chown(path, uid, gid);
        // }

        // fs::create_dir_all(pg_admin_dir)?;

        docker_compose_cmd()?.arg("up").run()?;

        Ok(())
    }
}
