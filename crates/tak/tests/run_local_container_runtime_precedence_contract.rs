mod support;

use std::fs;

use anyhow::Result;

use support::container_runtime::simulated_container_runtime_env;
use support::{run_tak_expect_success, write_tasks};

#[test]
fn run_command_prefers_task_runtime_over_workspace_default_when_infering_container_runtime()
-> Result<()> {
    let temp = tempfile::tempdir()?;
    fs::create_dir_all(temp.path().join("docker"))?;
    fs::write(temp.path().join("docker/Dockerfile"), "FROM alpine:3.20\n")?;
    write_tasks(
        temp.path(),
        r#"LOCAL = Local(id="dev", runtime=DockerfileRuntime(dockerfile=path("docker/Dockerfile")))

SPEC = module_spec(
  defaults={"container_runtime": ContainerRuntime("alpine:3.20")},
  tasks=[task(
    "check",
    steps=[cmd(
      "sh",
      "-c",
      "mkdir -p out && printf '%s\n' \"$TAK_RUNTIME_SOURCE\" > out/runtime-source.txt",
    )],
    execution=LocalOnly(LOCAL),
  )],
)
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
