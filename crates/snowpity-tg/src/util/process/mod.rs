use crate::prelude::*;
use crate::{fatal, Result};
use std::process::Stdio;

pub(crate) async fn run(program: &str, args: &[&str]) -> Result<Vec<u8>> {
    let display_args = shlex::try_join(args.iter().copied()).fatal_ctx(|| {
        format!("Couldn't run program that contains a nul byte: {program:?} {args:?}")
    })?;

    let display_cmd = format!("{program} {display_args}");
    debug!(
        cmd = %display_cmd,
        "Running program"
    );

    let output = tokio::process::Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .kill_on_drop(true)
        .output()
        .await
        .fatal_ctx(|| format!("Invocation failed. Command:\n`{display_cmd}`"))?;

    let status = output.status;

    if !status.success() {
        return Err(fatal!(
            "{program} invocation failed with status {status}. Command:\n{display_cmd}"
        ));
    }

    Ok(output.stdout)
}

async fn run_utf8(program: &str, args: &[&str]) -> Result<String> {
    let bytes = run(program, args).await?;
    std::str::from_utf8(&bytes)
        .fatal_ctx(|| {
            format!(
                "Bad output (invalid UTF-8).\n\
                Program: {program}.\n\
                Args: {args:?}.\n\
                Output: {bytes:?}.\n"
            )
        })
        .map(ToOwned::to_owned)
}

pub async fn run_json<T: serde::de::DeserializeOwned>(program: &str, args: &[&str]) -> Result<T> {
    let output = run_utf8(program, args).await?;
    serde_json::from_str(&output).fatal_ctx(|| {
        format!(
            "Bad output (invalid JSON).\n\
            Program: {program}.\n\
            Args: {args:?}.\n\
            Output: {output}.\n"
        )
    })
}
