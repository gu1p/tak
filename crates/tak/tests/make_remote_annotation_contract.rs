//! Black-box contract for Makefile-authored remote execution.

#![cfg(unix)]

use crate::support::direct_remote_runtime::{client_env, start_direct_agent};
use crate::support::make_runtime::{install_fake_make, start_local_daemon};
use crate::support::run_tak_output;

use std::fs;
use std::path::Path;

use anyhow::Result;

#[test]
fn make_annotations_select_remote_container_image() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    fs::create_dir_all(&workspace)?;
    install_fake_make(
        &workspace,
        r#"#!/bin/sh
printf 'goal=%s\nruntime=%s\nimage=%s\n' \
  "$1" "$TAK_REMOTE_RUNTIME" "$TAK_REMOTE_CONTAINER_IMAGE"
"#,
    )?;
    fs::write(
        workspace.join("Makefile"),
        "# tak: execution=remote\n\
         # tak: container-image=alpine:3.20\n\
         check:\n\
         \t@printf 'local Make recipe must not run\\n' >&2\n\
         \t@exit 91\n",
    )?;

    let mut environment = client_env(temp.path());
    environment.insert("PATH".into(), "bin:/usr/bin:/bin".into());
    let _daemon = start_local_daemon(&workspace, &mut environment);
    let daemon_socket = environment
        .get("TAKD_SOCKET")
        .expect("local daemon socket")
        .clone();
    let _agent = start_direct_agent(
        temp.path(),
        &workspace,
        "make-annotation-remote",
        Path::new(&daemon_socket),
    );
    let output = run_tak_output(
        &workspace,
        &["make", "check", "--pass-env", "PATH"],
        &environment,
    )?;

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
    assert!(stdout.contains("image=alpine:3.20"), "stdout:\n{stdout}");
    Ok(())
}
