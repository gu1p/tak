//! Black-box contracts for local container run-mode overrides.

use crate::support;

use std::fs;

use anyhow::Result;

use support::container_runtime::simulated_container_runtime_env;
use support::{run_tak_expect_success, write_tasks};

#[test]
fn run_command_uses_tasks_default_container_runtime_with_local_container_flags() -> Result<()> {
    let temp = tempfile::tempdir()?;
    fs::create_dir_all(temp.path().join("docker"))?;
    fs::write(temp.path().join("docker/Dockerfile"), "FROM alpine:3.20\n")?;
    write_tasks(
        temp.path(),
        r#"SPEC = module_spec(defaults={"container_runtime": Runtime.Dockerfile(path("docker/Dockerfile"))}, tasks=[task("check", steps=[cmd("sh", "-c", "mkdir -p out && printf '%s\n' \"$TAK_RUNTIME_SOURCE\" > out/runtime-source.txt")])])
SPEC
"#,
    )?;

    let env = simulated_container_runtime_env(temp.path());
    let stdout = run_tak_expect_success(
        temp.path(),
        &["run", "--local", "--container", "check"],
        &env,
    )?;

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

#[test]
fn run_command_uses_explicit_container_dockerfile_flag() -> Result<()> {
    let temp = tempfile::tempdir()?;
    fs::create_dir_all(temp.path().join("docker"))?;
    fs::write(temp.path().join("docker/Dockerfile"), "FROM alpine:3.20\n")?;
    write_tasks(
        temp.path(),
        r#"SPEC = module_spec(tasks=[task("check", steps=[cmd("sh", "-c", "mkdir -p out && printf '%s\n' \"$TAK_RUNTIME_SOURCE\" > out/runtime-source.txt")])])
SPEC
"#,
    )?;

    let env = simulated_container_runtime_env(temp.path());
    let stdout = run_tak_expect_success(
        temp.path(),
        &[
            "run",
            "--local",
            "--container",
            "--container-dockerfile",
            "docker/Dockerfile",
            "check",
        ],
        &env,
    )?;

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
