#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::Path;
use std::process::{Command as StdCommand, Output};

use anyhow::{Context, Result, bail};

fn run_tak(
    workspace_root: &Path,
    args: &[&str],
    extra_env: &BTreeMap<String, String>,
) -> Result<Output> {
    let mut command = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"));
    command
        .current_dir(workspace_root)
        .args(args)
        .env("TAKD_SOCKET", workspace_root.join(".missing-takd.sock"));
    for (key, value) in extra_env {
        command.env(key, value);
    }
    command
        .output()
        .with_context(|| format!("failed running `tak {}`", args.join(" ")))
}

pub fn run_tak_expect_success(
    workspace_root: &Path,
    args: &[&str],
    extra_env: &BTreeMap<String, String>,
) -> Result<String> {
    let output = run_tak(workspace_root, args, extra_env)?;
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

pub fn run_tak_expect_failure(
    workspace_root: &Path,
    args: &[&str],
    extra_env: &BTreeMap<String, String>,
) -> Result<(String, String)> {
    let output = run_tak(workspace_root, args, extra_env)?;
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
