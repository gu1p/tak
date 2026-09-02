//! Black-box contract for CLI placement and runtime precedence over Make annotations.

#![cfg(unix)]

use crate::support::direct_remote_runtime::{client_env, start_direct_agent};
use crate::support::make_runtime::{install_fake_make, start_local_daemon};
use crate::support::run_tak_output;

use std::fs;
use std::path::Path;

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
        &workspace,
        "#!/bin/sh\nprintf 'goal=%s\\nruntime=%s\\nsource=%s\\n' \
         \"$1\" \"$TAK_REMOTE_RUNTIME\" \"$TAK_RUNTIME_SOURCE\"\n",
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
        "make-cli-override-remote",
        Path::new(&daemon_socket),
    );

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
            "--pass-env",
            "PATH",
        ],
        &environment,
    )?;

    assert!(output.status.success(), "status: {:?}", output.status);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("goal=check\nruntime=containerized\nsource=dockerfile\n"),
        "{stdout}"
    );
    Ok(())
}
