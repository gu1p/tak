//! Black-box contract for remote container runtime overrides.

use crate::support;

use std::fs;

use anyhow::Result;

use support::direct_remote_runtime::{client_env, start_direct_agent};
use support::{run_tak_expect_success, write_tasks};

#[test]
fn run_command_uses_tasks_default_container_runtime_with_remote_flag() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace_root = temp.path().join("workspace");
    let _agent = start_direct_agent(temp.path(), &workspace_root, "override-remote-builder");
    write_tasks(
        &workspace_root,
        r#"SPEC = module_spec(defaults={"container_runtime": Runtime.Image("alpine:3.20")}, tasks=[task("check", outputs=[path("out")], steps=[cmd("sh", "-c", "mkdir -p out && printf '%s\n' \"$TAK_RUNTIME_SOURCE\" > out/runtime-source.txt")])])
SPEC
"#,
    )?;

    let stdout = run_tak_expect_success(
        &workspace_root,
        &["run", "--remote", "check"],
        &client_env(temp.path()),
    )?;

    assert!(stdout.contains("placement=remote"), "stdout:\n{stdout}");
    assert!(
        stdout.contains("runtime=containerized"),
        "stdout:\n{stdout}"
    );
    assert_eq!(
        fs::read_to_string(workspace_root.join("out/runtime-source.txt"))?.trim(),
        "image"
    );
    Ok(())
}

#[test]
fn run_command_accepts_remote_dockerfile_override_without_container_flag() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace_root = temp.path().join("workspace");
    fs::create_dir_all(workspace_root.join("docker"))?;
    fs::write(
        workspace_root.join("docker/Dockerfile"),
        "FROM alpine:3.20\n",
    )?;
    let _agent = start_direct_agent(temp.path(), &workspace_root, "override-remote-dockerfile");
    write_tasks(
        &workspace_root,
        r#"SPEC = module_spec(tasks=[task("check", outputs=[path("out")], steps=[cmd("sh", "-c", "mkdir -p out && printf '%s\n' \"$TAK_RUNTIME_SOURCE\" > out/runtime-source.txt")])])
SPEC
"#,
    )?;

    let stdout = run_tak_expect_success(
        &workspace_root,
        &[
            "run",
            "--remote",
            "--container-dockerfile",
            "docker/Dockerfile",
            "check",
        ],
        &client_env(temp.path()),
    )?;

    assert!(stdout.contains("placement=remote"), "stdout:\n{stdout}");
    assert_eq!(
        fs::read_to_string(workspace_root.join("out/runtime-source.txt"))?.trim(),
        "dockerfile"
    );
    Ok(())
}
