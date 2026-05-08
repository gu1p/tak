use std::collections::BTreeMap;

use anyhow::Result;

use crate::support::{run_tak_expect_failure, run_tak_output};

#[test]
fn docker_build_is_rejected_with_tak_execution_guidance() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let (_stdout, stderr) = run_tak_expect_failure(
        temp.path(),
        &["docker", "build", "-t", "demo", "."],
        &BTreeMap::new(),
    )?;

    assert!(stderr.contains("tak docker build is not supported"));
    assert!(stderr.contains("tak docker run -f Dockerfile"));
    Ok(())
}

#[test]
fn docker_run_rejects_detach() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let (_stdout, stderr) = run_tak_expect_failure(
        temp.path(),
        &["docker", "run", "--detach", "alpine:3.20", "true"],
        &BTreeMap::new(),
    )?;

    assert!(stderr.contains("tak docker run does not support detached containers"));
    Ok(())
}

#[test]
fn docker_run_rejects_publish_until_forwarding_exists() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let (_stdout, stderr) = run_tak_expect_failure(
        temp.path(),
        &["docker", "run", "-p", "8080:80", "alpine:3.20", "true"],
        &BTreeMap::new(),
    )?;

    assert!(stderr.contains("tak docker run does not support port publishing yet"));
    Ok(())
}

#[test]
fn docker_run_defaults_to_remote_and_reports_missing_inventory() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let mut env = BTreeMap::new();
    env.insert(
        "XDG_CONFIG_HOME".to_string(),
        temp.path().join("config").display().to_string(),
    );

    let (_stdout, stderr) =
        run_tak_expect_failure(temp.path(), &["docker", "run", "alpine:3.20", "true"], &env)?;

    assert!(stderr.contains("no configured remote agents match tak docker run"));
    Ok(())
}

#[test]
fn docker_run_accepts_global_local_selector_before_docker() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let output = run_tak_output(
        temp.path(),
        &[
            "--local",
            "docker",
            "run",
            "--detach",
            "alpine:3.20",
            "true",
        ],
        &BTreeMap::new(),
    )?;

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("tak docker run does not support detached containers"));
    Ok(())
}
