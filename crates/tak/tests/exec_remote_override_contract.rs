//! Black-box contract for `tak exec` remote runtime overrides.

use crate::support;

use std::fs;

use anyhow::Result;

use support::direct_remote_runtime::{client_env, start_direct_agent};
use support::run_tak_output;

#[test]
fn exec_supports_remote_container_image_override_without_container_flag() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let _agent = start_direct_agent(temp.path(), temp.path(), "exec-remote-image");

    let output = run_tak_output(
        temp.path(),
        &[
            "exec",
            "--remote",
            "--container-image",
            "alpine:3.20",
            "--",
            "sh",
            "-c",
            "printf '%s\\n' \"$TAK_RUNTIME_SOURCE\"",
        ],
        &client_env(temp.path()),
    )?;

    assert!(output.status.success(), "status: {:?}", output.status);
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "image");
    Ok(())
}

#[test]
fn exec_supports_remote_dockerfile_override_without_container_flag() -> Result<()> {
    let temp = tempfile::tempdir()?;
    fs::create_dir_all(temp.path().join("docker"))?;
    fs::write(temp.path().join("docker/Dockerfile"), "FROM alpine:3.20\n")?;
    let _agent = start_direct_agent(temp.path(), temp.path(), "exec-remote-dockerfile");

    let output = run_tak_output(
        temp.path(),
        &[
            "exec",
            "--remote",
            "--container-dockerfile",
            "docker/Dockerfile",
            "--",
            "sh",
            "-c",
            "printf '%s\\n' \"$TAK_RUNTIME_SOURCE\"",
        ],
        &client_env(temp.path()),
    )?;

    assert!(output.status.success(), "status: {:?}", output.status);
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "dockerfile");
    Ok(())
}

#[test]
fn exec_warns_that_remote_container_flag_is_redundant() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let _agent = start_direct_agent(temp.path(), temp.path(), "exec-remote-redundant");

    let output = run_tak_output(
        temp.path(),
        &[
            "exec",
            "--remote",
            "--container",
            "--container-image",
            "alpine:3.20",
            "--",
            "sh",
            "-c",
            "printf '%s\\n' \"$TAK_RUNTIME_SOURCE\"",
        ],
        &client_env(temp.path()),
    )?;

    assert!(output.status.success(), "status: {:?}", output.status);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(
            "warning: --container is redundant with --remote; remote execution already implies a containerized runtime"
        ),
        "stderr:\n{stderr}"
    );
    Ok(())
}
