use std::collections::BTreeMap;

use anyhow::Result;

use crate::support::{run_tak_output, task_history};

#[test]
fn docker_ps_does_not_list_remote_task_history_as_local_container() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let state_root = temp.path().join("state");
    task_history::write_active_remote_container_run(&state_root);

    let mut env = BTreeMap::new();
    env.insert(
        "XDG_STATE_HOME".to_string(),
        state_root.display().to_string(),
    );
    env.insert(
        "XDG_CONFIG_HOME".to_string(),
        temp.path().join("config").display().to_string(),
    );

    let output = run_tak_output(temp.path(), &["--local", "docker", "ps"], &env)?;

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Tak Containers"), "stdout:\n{stdout}");
    assert!(!stdout.contains("node=local"), "stdout:\n{stdout}");
    assert!(!stdout.contains("remote-task-run-1"), "stdout:\n{stdout}");
    Ok(())
}
