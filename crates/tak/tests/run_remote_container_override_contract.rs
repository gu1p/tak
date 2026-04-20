//! Black-box contract for remote container run-mode overrides.

use crate::support;

use std::collections::BTreeMap;
use std::fs;

use anyhow::Result;

use support::container_runtime::simulated_container_runtime_env;
use support::live_direct::{LiveDirectRoots, init_direct_agent, spawn_direct_agent_with_env};
use support::live_direct_remote::add_remote;
use support::live_direct_token::wait_for_token;
use support::tor_smoke::takd_bin;
use support::{run_tak_expect_success, write_tasks};

#[test]
fn run_command_uses_tasks_default_container_runtime_with_remote_container_flags() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace_root = temp.path().join("workspace");
    let remote_exec_root = temp.path().join("remote-exec");
    let roots = LiveDirectRoots::new(temp.path());
    fs::create_dir_all(workspace_root.join("docker"))?;
    fs::write(
        workspace_root.join("docker/Dockerfile"),
        "FROM alpine:3.20\n",
    )?;
    write_tasks(
        &workspace_root,
        r#"SPEC = module_spec(defaults={"container_runtime": DockerfileRuntime(dockerfile=path("docker/Dockerfile"))}, tasks=[task("check", outputs=[path("out")], steps=[cmd("sh", "-c", "mkdir -p out && printf '%s\n' \"$TAK_RUNTIME_SOURCE\" > out/runtime-source.txt")])])
SPEC
"#,
    )?;

    let takd = takd_bin();
    init_direct_agent(&takd, &roots, "override-remote-container-builder");
    let mut serve_env = simulated_container_runtime_env(temp.path())
        .into_iter()
        .collect::<Vec<_>>();
    serve_env.push((
        "TAKD_REMOTE_EXEC_ROOT".to_string(),
        remote_exec_root.display().to_string(),
    ));
    let _agent = spawn_direct_agent_with_env(&takd, &roots, &serve_env);
    let token = wait_for_token(&takd, &roots);
    add_remote(&workspace_root, &roots, &token);

    let mut env = BTreeMap::new();
    env.insert(
        "XDG_CONFIG_HOME".to_string(),
        roots.client_config_root.display().to_string(),
    );
    let stdout = run_tak_expect_success(
        &workspace_root,
        &["run", "--remote", "--container", "check"],
        &env,
    )?;

    assert!(stdout.contains("placement=remote"), "stdout:\n{stdout}");
    assert!(
        stdout.contains("remote_node=override-remote-container-builder"),
        "stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("runtime=containerized"),
        "stdout:\n{stdout}"
    );
    assert_eq!(
        fs::read_to_string(workspace_root.join("out/runtime-source.txt"))?.trim(),
        "dockerfile"
    );
    assert!(
        !remote_exec_root.exists() || remote_exec_root.read_dir()?.next().is_none(),
        "finished remote execution roots should be cleaned: {}",
        remote_exec_root.display()
    );
    Ok(())
}
