use std::collections::BTreeMap;

use anyhow::Result;

use crate::support::{run_tak_expect_failure, run_tak_output};

#[test]
fn docker_ps_does_not_list_failed_local_container_setup() -> Result<()> {
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
    env.insert(
        "TAK_TEST_CONTAINER_LIFECYCLE_FAILURES".to_string(),
        "local:start".to_string(),
    );

    let (_stdout, stderr) = run_tak_expect_failure(
        temp.path(),
        &["--local", "docker", "run", "alpine:3.20", "true"],
        &env,
    )?;
    assert!(stderr.contains("container lifecycle start failed"));

    let output = run_tak_output(temp.path(), &["--local", "docker", "ps"], &env)?;
    assert!(output.status.success(), "status: {:?}", output.status);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("(none)"), "stdout:\n{stdout}");
    assert!(!stdout.contains("kind=docker-run"), "stdout:\n{stdout}");
    Ok(())
}
