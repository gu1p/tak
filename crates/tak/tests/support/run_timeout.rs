#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::Path;
use std::process::{Command, Output};
use std::time::Duration;

use anyhow::{Context, Result, bail};

use super::run::tak_bin;

#[path = "bounded_process.rs"]
mod bounded_process;
pub use bounded_process::output_with_timeout;

fn run_tak_output(
    workspace_root: &Path,
    args: &[&str],
    extra_env: &BTreeMap<String, String>,
    timeout: Duration,
) -> Result<Output> {
    let mut command = Command::new(tak_bin());
    command
        .current_dir(workspace_root)
        .args(args)
        .env("TAKD_SOCKET", workspace_root.join(".missing-takd.sock"));
    command.envs(extra_env);
    output_with_timeout(command, timeout)
        .with_context(|| format!("failed running `tak {}`", args.join(" ")))
}

pub fn run_tak_expect_success_with_timeout(
    workspace_root: &Path,
    args: &[&str],
    extra_env: &BTreeMap<String, String>,
    timeout: Duration,
) -> Result<String> {
    let output = run_tak_output(workspace_root, args, extra_env, timeout)?;
    if !output.status.success() {
        bail!(
            "command `tak {}` failed\nstdout:\n{}\nstderr:\n{}",
            args.join(" "),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn run_tak_expect_failure_with_timeout(
    workspace_root: &Path,
    args: &[&str],
    extra_env: &BTreeMap<String, String>,
    timeout: Duration,
) -> Result<(String, String)> {
    let output = run_tak_output(workspace_root, args, extra_env, timeout)?;
    if output.status.success() {
        bail!(
            "command `tak {}` unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
            args.join(" "),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok((
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    ))
}
