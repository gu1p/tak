//! Black-box contracts for local run-mode overrides.

mod support;

use std::collections::BTreeMap;
use std::fs;

use anyhow::Result;

use support::container_runtime::simulated_container_runtime_env;
use support::{run_tak_expect_failure, run_tak_expect_success, write_tasks};

#[test]
fn run_command_rejects_container_without_mode_selector() -> Result<()> {
    let temp = tempfile::tempdir()?;
    write_tasks(
        temp.path(),
        "SPEC = module_spec(tasks=[task(\"check\", steps=[cmd(\"echo\", \"ok\")])])\nSPEC\n",
    )?;
    let env = BTreeMap::new();
    let (_stdout, stderr) =
        run_tak_expect_failure(temp.path(), &["run", "--container", "check"], &env)?;

    assert!(
        stderr.contains("--container requires exactly one of --local or --remote"),
        "stderr:\n{stderr}"
    );
    Ok(())
}

#[test]
fn run_command_executes_neutral_task_locally_with_local_flag() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let marker = temp.path().join("out/local-run.txt");
    write_tasks(
        temp.path(),
        &format!(
            "SPEC = module_spec(tasks=[task(\"check\", steps=[cmd(\"sh\", \"-c\", \"mkdir -p out && echo local-run > {marker}\")])])\nSPEC\n",
            marker = marker.display()
        ),
    )?;
    let env = BTreeMap::new();
    let stdout = run_tak_expect_success(temp.path(), &["run", "--local", "check"], &env)?;

    assert!(stdout.contains("//:check: ok"), "stdout:\n{stdout}");
    assert!(stdout.contains("placement=local"), "stdout:\n{stdout}");
    assert_eq!(fs::read_to_string(marker)?.trim(), "local-run");
    Ok(())
}

#[test]
fn run_command_local_flag_preserves_declared_remote_container_runtime() -> Result<()> {
    let temp = tempfile::tempdir()?;
    fs::create_dir_all(temp.path().join("docker"))?;
    fs::write(temp.path().join("docker/Dockerfile"), "FROM alpine:3.20\n")?;
    write_tasks(
        temp.path(),
        r#"REMOTE = Remote(runtime=DockerfileRuntime(dockerfile=path("docker/Dockerfile")))

SPEC = module_spec(tasks=[task(
  "check",
  steps=[cmd("sh", "-c", "mkdir -p out && printf '%s\n' \"$TAK_RUNTIME_SOURCE\" > out/runtime-source.txt")],
  execution=RemoteOnly(REMOTE),
)])
SPEC
"#,
    )?;

    let env = simulated_container_runtime_env(temp.path());
    let stdout = run_tak_expect_success(temp.path(), &["run", "--local", "check"], &env)?;

    assert!(stdout.contains("placement=local"), "stdout:\n{stdout}");
    assert!(
        stdout.contains("runtime=containerized"),
        "stdout:\n{stdout}"
    );
    assert_eq!(
        fs::read_to_string(temp.path().join("out/runtime-source.txt"))?.trim(),
        "dockerfile"
    );
    Ok(())
}
