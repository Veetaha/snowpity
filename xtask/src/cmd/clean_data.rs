use crate::util::{fs, repo_abs_path};
use clap::Parser;

/// Clean the data directories of databases that are mapped into the development
/// containers. This doesn't clear the data of the database clients, that merely
/// store configuration files, which are generally better kept persistent.
#[derive(Parser, Debug)]
pub struct CleanData {}

impl crate::cmd::Cmd for CleanData {
    fn run(self) -> anyhow::Result<()> {
        fs::remove_dir_all_if_exists(&repo_abs_path(["data", "postgres"]))?;
        Ok(())
    }
}
