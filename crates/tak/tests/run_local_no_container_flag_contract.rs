//! Black-box contracts for `tak run --local-no-container`.

use crate::support::{run_tak_expect_failure, run_tak_expect_success, write_tasks};

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::Result;

#[test]
fn run_command_local_no_container_ignores_remote_dockerfile_runtime() -> Result<()> {
    let temp = tempfile::tempdir()?;
    fs::create_dir_all(temp.path().join("docker"))?;
    fs::write(temp.path().join("docker/Dockerfile"), "FROM alpine:3.20\n")?;
    write_tasks(
        temp.path(),
        r#"EXEC = Execution.FirstAvailable([
  Execution.Remote(container=Container.Dockerfile(path("docker/Dockerfile"))),
  Execution.Local(),
])

SPEC = module_spec(tasks=[task("check", steps=[cmd("sh", "-c", "mkdir -p out && printf '%s\n' \"$TAK_RUNTIME_SOURCE\" > out/runtime.txt")], execution=EXEC)])
SPEC
"#,
    )?;
    assert_host_runtime(temp.path())
}

#[test]
fn run_command_local_no_container_ignores_default_container_runtime() -> Result<()> {
    let temp = tempfile::tempdir()?;
    write_tasks(
        temp.path(),
        r#"SPEC = module_spec(
  defaults=Defaults(container=Container.Image("alpine:3.20")),
  tasks=[task("check", steps=[cmd("sh", "-c", "mkdir -p out && printf '%s\n' \"$TAK_RUNTIME_SOURCE\" > out/runtime.txt")])],
)
SPEC
"#,
    )?;
    assert_host_runtime(temp.path())
}

#[test]
fn run_command_local_no_container_rejects_remote_and_container_flags() -> Result<()> {
    let temp = tempfile::tempdir()?;
    write_tasks(
        temp.path(),
        "SPEC = module_spec(tasks=[task(\"check\")])\nSPEC\n",
    )?;
    assert_rejected(
        temp.path(),
        &["run", "--local-no-container", "--remote", "check"],
        "--local-no-container and --remote are mutually exclusive",
    )?;
    assert_rejected(
        temp.path(),
        &["run", "--local-no-container", "--container", "check"],
        "--local-no-container and --container are mutually exclusive",
    )?;
    assert_rejected(
        temp.path(),
        &[
            "run",
            "--local-no-container",
            "--container-image",
            "alpine:3.20",
            "check",
        ],
        "--local-no-container cannot be combined with container source flags",
    )
}

fn assert_host_runtime(workspace: &Path) -> Result<()> {
    let env = BTreeMap::from([("TAK_RUNTIME_SOURCE".to_string(), "none".to_string())]);
    let stdout =
        run_tak_expect_success(workspace, &["run", "--local-no-container", "check"], &env)?;
    assert!(stdout.contains("placement=local"), "stdout:\n{stdout}");
    assert!(stdout.contains("runtime=none"), "stdout:\n{stdout}");
    assert_eq!(
        fs::read_to_string(workspace.join("out/runtime.txt"))?.trim(),
        "none"
    );
    Ok(())
}

fn assert_rejected(workspace: &Path, args: &[&str], expected: &str) -> Result<()> {
    let (_stdout, stderr) = run_tak_expect_failure(workspace, args, &BTreeMap::new())?;
    assert!(stderr.contains(expected), "stderr:\n{stderr}");
    Ok(())
}
