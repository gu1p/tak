use anyhow::Result;

use super::ps_process::{ChildCleanup, spawn_tak_child, wait_for_docker_ps};
use crate::support::direct_remote_runtime::{client_env, start_direct_agent};
use crate::support::{run_tak_output, write_tasks};

#[test]
fn docker_ps_does_not_list_remote_task_history_as_local_container() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace_root = temp.path().join("workspace");
    let _agent = start_direct_agent(temp.path(), &workspace_root, "remote-history-builder");
    write_tasks(
        &workspace_root,
        r#"SPEC = module_spec(tasks=[task("check", steps=[cmd("sh", "-c", "sleep 10")])])
SPEC
"#,
    )?;

    let mut env = client_env(temp.path());
    env.insert(
        "XDG_STATE_HOME".to_string(),
        temp.path().join("state").display().to_string(),
    );
    let mut child = spawn_tak_child(
        &workspace_root,
        &[
            "run",
            "--remote",
            "--container-image",
            "alpine:3.20",
            "check",
        ],
        &env,
    )?;
    let _guard = ChildCleanup(&mut child);

    let _remote_stdout = wait_for_docker_ps(
        &workspace_root,
        &["docker", "ps"],
        &env,
        "node=remote-history-builder kind=task",
    )?;

    let local_output = run_tak_output(&workspace_root, &["--local", "docker", "ps"], &env)?;
    assert!(local_output.status.success());
    let local_stdout = String::from_utf8_lossy(&local_output.stdout);
    assert!(!local_stdout.contains("node=local"));
    Ok(())
}
