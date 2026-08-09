//! Black-box contract for CLI placement and runtime precedence over Make annotations.

#![cfg(unix)]

use crate::support::direct_remote_runtime::{client_env, start_direct_agent};
use crate::support::make_runtime::install_fake_make;
use crate::support::run_tak_output;

use std::fs;

use anyhow::Result;

#[test]
fn remote_dockerfile_cli_override_wins_over_local_image_annotations() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    fs::create_dir_all(workspace.join("docker"))?;
    fs::write(workspace.join("docker/Dockerfile"), "FROM alpine:3.20\n")?;
    fs::write(
        workspace.join("Makefile"),
        "# tak: execution=local\n\
         # tak: container-image=busybox:1.36\n\
         check:\n\
         \t@exit 91\n",
    )?;
    install_fake_make(
        temp.path(),
        "#!/bin/sh\nprintf 'goal=%s\\nruntime=%s\\nsource=%s\\n' \
         \"$1\" \"$TAK_REMOTE_RUNTIME\" \"$TAK_RUNTIME_SOURCE\"\n",
    )?;
    let _agent = start_direct_agent(temp.path(), &workspace, "make-cli-override-remote");

    let output = run_tak_output(
        &workspace,
        &[
            "make",
            "--remote",
            "--container-dockerfile",
            "docker/Dockerfile",
            "--container-build-context",
            ".",
            "check",
        ],
        &client_env(temp.path()),
    )?;

    assert!(output.status.success(), "status: {:?}", output.status);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "goal=check\nruntime=containerized\nsource=dockerfile\n"
    );
    Ok(())
}
