use std::collections::BTreeMap;

use anyhow::Result;

use crate::support::run_tak_output;

#[test]
fn docker_ps_renders_empty_state_without_legacy_container_history() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let mut env = BTreeMap::new();
    env.insert(
        "XDG_STATE_HOME".to_string(),
        temp.path().join("state").display().to_string(),
    );
    env.insert(
        "XDG_CONFIG_HOME".to_string(),
        temp.path().join("config").display().to_string(),
    );
    let output = run_tak_output(temp.path(), &["--local", "docker", "ps"], &env)?;
    assert!(output.status.success(), "status: {:?}", output.status);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("(none)"), "stdout:\n{stdout}");
    assert!(!stdout.contains("kind=docker-run"), "stdout:\n{stdout}");
    Ok(())
}
