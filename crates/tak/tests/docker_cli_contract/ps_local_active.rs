use anyhow::Result;

use crate::support::{run_tak_output, task_history};

#[test]
fn docker_ps_lists_read_only_legacy_local_container_history() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let state_root = temp.path().join("state");
    task_history::write_active_container_run(&state_root);
    let mut env = std::collections::BTreeMap::new();
    env.insert(
        "XDG_STATE_HOME".to_string(),
        state_root.display().to_string(),
    );
    env.insert(
        "XDG_CONFIG_HOME".to_string(),
        temp.path().join("config").display().to_string(),
    );
    let output = run_tak_output(temp.path(), &["--local", "docker", "ps"], &env)?;
    assert!(output.status.success(), "status: {:?}", output.status);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("node=local"), "stdout:\n{stdout}");
    assert!(stdout.contains("kind=task"), "stdout:\n{stdout}");
    assert!(
        stdout.contains("source=image:alpine:3.20"),
        "stdout:\n{stdout}"
    );
    assert!(stdout.contains("command=make build"), "stdout:\n{stdout}");
    Ok(())
}
