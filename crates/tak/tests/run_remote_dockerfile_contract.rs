//! Black-box contract for Dockerfile-backed remote task execution.

mod support;

use std::fs;

use anyhow::Result;

use support::container_runtime::simulated_container_runtime_env;
use support::live_direct::{LiveDirectRoots, init_direct_agent, spawn_direct_agent_with_env};
use support::live_direct_remote::add_remote;
use support::live_direct_token::wait_for_token;
use support::run_tak_expect_success;
use support::tor_smoke::takd_bin;

#[test]
fn run_remote_dockerfile_runtime_reports_containerized_summary() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace_root = temp.path().join("workspace");
    let roots = LiveDirectRoots::new(temp.path());
    fs::create_dir_all(workspace_root.join("docker"))?;
    fs::write(
        workspace_root.join("docker/Dockerfile"),
        "FROM alpine:3.20\nRUN printf 'built\\n' > /tmp/built.txt\n",
    )?;
    fs::write(
        workspace_root.join("TASKS.py"),
        r#"
REMOTE = Remote(
  pool="build",
  required_tags=["builder"],
  required_capabilities=["linux"],
  runtime=DockerfileRuntime(dockerfile=path("docker/Dockerfile")),
)

SPEC = module_spec(tasks=[
  task(
    "remote_container",
    outputs=[path("out")],
    steps=[cmd("sh", "-c", "mkdir -p out && printf '%s\n' \"$TAK_RUNTIME_SOURCE\" > out/runtime-source.txt")],
    execution=RemoteOnly(REMOTE),
  ),
])
SPEC
"#,
    )?;

    let takd = takd_bin();
    init_direct_agent(&takd, &roots, "remote-dockerfile-builder");
    let serve_env = simulated_container_runtime_env(temp.path())
        .into_iter()
        .collect::<Vec<_>>();
    let _agent = spawn_direct_agent_with_env(&takd, &roots, &serve_env);
    let token = wait_for_token(&takd, &roots);
    add_remote(&workspace_root, &roots, &token);

    let mut env = std::collections::BTreeMap::new();
    env.insert(
        "XDG_CONFIG_HOME".to_string(),
        roots.client_config_root.display().to_string(),
    );

    let stdout = run_tak_expect_success(&workspace_root, &["run", "//:remote_container"], &env)?;

    assert!(stdout.contains("placement=remote"), "stdout:\n{stdout}");
    assert!(
        stdout.contains("remote_node=remote-dockerfile-builder"),
        "stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("runtime=containerized"),
        "stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("runtime_engine=docker"),
        "stdout:\n{stdout}"
    );
    assert_eq!(
        fs::read_to_string(workspace_root.join("out/runtime-source.txt"))?.trim(),
        "dockerfile"
    );
    Ok(())
}
