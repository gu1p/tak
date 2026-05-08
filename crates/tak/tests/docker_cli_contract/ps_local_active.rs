use anyhow::Result;

use super::ps_process::{ChildCleanup, spawn_tak_child, wait_for_docker_ps};
use crate::support;

#[test]
fn docker_ps_lists_active_local_docker_run_from_task_history() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let mut env = support::container_runtime::simulated_container_runtime_env(temp.path());
    env.insert(
        "XDG_STATE_HOME".to_string(),
        temp.path().join("state").display().to_string(),
    );
    env.insert(
        "XDG_CONFIG_HOME".to_string(),
        temp.path().join("config").display().to_string(),
    );
    let mut child = spawn_tak_child(
        temp.path(),
        &[
            "--local",
            "docker",
            "run",
            "alpine:3.20",
            "sh",
            "-c",
            "sleep 10",
        ],
        &env,
    )?;
    let _guard = ChildCleanup(&mut child);

    let stdout = wait_for_docker_ps(
        temp.path(),
        &["--local", "docker", "ps"],
        &env,
        "kind=docker-run",
    )?;
    assert!(stdout.contains("node=local"), "stdout:\n{stdout}");
    assert!(
        stdout.contains("source=image:alpine:3.20"),
        "stdout:\n{stdout}"
    );
    assert!(stdout.contains("command=sh -c"), "stdout:\n{stdout}");
    Ok(())
}
