pub(crate) mod fs;

use anyhow::Result;
use devx_cmd::cmd;
use std::path::{Path, PathBuf};

pub(crate) fn repo_abs_path<I>(components: I) -> PathBuf
where
    I: IntoIterator,
    I::Item: AsRef<Path>,
{
    let mut path = repo_root();
    path.extend(components);
    path
}

pub(crate) fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_owned()
}

// FIXME: remove this commentted code
// TODO: when adding pgadmin support, we need to create the pgadmin user on host
// to be able to bind map pgadmin's configuration files
//
// fn linux_user_exists(username: &str) -> Result<bool> {
//     let code = std::process::Command::new("id")
//         .arg("-u")
//         .arg(username)
//         .stdin(Stdio::null())
//         .stdout(Stdio::null())
//         .stderr(Stdio::null())
//         .status()?
//         .code();

//     match code {
//         Some(0) => Ok(true),
//         Some(1) => Ok(false),
//         _ => anyhow::bail!("unexpected exit code from `id`: {code:?}"),
//     }
// }

// #[instrument]
// fn create_linux_user_if_not_exists(username: &str) -> Result<()> {
//     if linux_user_exists(username)? {
//         tracing::debug!("user already exists");
//         return Ok(());
//     }

//     run!("useradd", username)?;

//     Ok(())
// }

fn read_silent_trimmed(cmd: devx_cmd::Cmd) -> Result<String> {
    Ok({ cmd }.log_cmd(None).read()?.trim().to_string())
}

pub(crate) fn docker_compose_cmd() -> Result<devx_cmd::Cmd> {
    let uid = read_silent_trimmed(cmd!("id", "--user"))?;
    let gid = read_silent_trimmed(cmd!("id", "--group"))?;

    let mut cmd = cmd!("docker", "compose");
    cmd.current_dir(repo_root());
    cmd.env("CURRENT_UID", format!("{uid}:{gid}"));

    Ok(cmd)
}
