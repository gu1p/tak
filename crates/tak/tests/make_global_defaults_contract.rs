//! Black-box contract for global Makefile execution defaults.

#![cfg(unix)]

use crate::support::direct_remote_runtime::{client_env, start_direct_agent};
use crate::support::make_runtime::install_fake_make;
use crate::support::run_tak_output;

use std::fs;

use anyhow::Result;

#[test]
fn global_defaults_apply_and_goal_annotations_override_them() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    fs::create_dir_all(&workspace)?;
    install_fake_make(
        temp.path(),
        r#"#!/bin/sh
printf 'goal=%s\nruntime=%s\nimage=%s\n' \
  "$1" "$TAK_REMOTE_RUNTIME" "$TAK_REMOTE_CONTAINER_IMAGE"
"#,
    )?;
    fs::write(
        workspace.join("Makefile"),
        "# tak: default.execution=remote\n\
         # tak: default.container-image=alpine:3.20\n\
         # tak: container-image=debian:bookworm\n\
         check:\n\
         \t@printf 'local Make recipe must not run\\n' >&2\n\
         \t@exit 91\n",
    )?;

    let _agent = start_direct_agent(temp.path(), &workspace, "make-global-defaults");
    let output = run_tak_output(&workspace, &["make", "check"], &client_env(temp.path()))?;

    assert!(
        output.status.success(),
        "status: {:?}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("goal=check"), "stdout:\n{stdout}");
    assert!(
        stdout.contains("runtime=containerized"),
        "stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("image=debian:bookworm"),
        "stdout:\n{stdout}"
    );
    assert!(
        !String::from_utf8_lossy(&output.stderr).contains("no Tak execution configuration"),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
