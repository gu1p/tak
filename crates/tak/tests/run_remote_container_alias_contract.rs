//! Black-box contract for `--remote --container` compatibility.

use crate::support;

use std::collections::BTreeMap;
use std::fs;

use anyhow::Result;

use support::container_runtime::simulated_container_runtime_env;
use support::live_direct::{LiveDirectRoots, init_direct_agent, spawn_direct_agent_with_env};
use support::live_direct_remote::add_remote;
use support::live_direct_token::wait_for_token;
use support::run_tak_output;
use support::tor_smoke::takd_bin;
use support::write_tasks;

#[test]
fn run_command_warns_that_remote_container_flag_is_redundant() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace_root = temp.path().join("workspace");
    let remote_exec_root = temp.path().join("remote-exec");
    let roots = LiveDirectRoots::new(temp.path());
    write_tasks(
        &workspace_root,
        r#"SPEC = module_spec(defaults=Defaults(container_runtime=Runtime.Dockerfile(path("docker/Dockerfile"))), tasks=[task("check", outputs=[path("out")], steps=[cmd("sh", "-c", "mkdir -p out && printf '%s\n' \"$TAK_RUNTIME_SOURCE\" > out/runtime-source.txt")])])
SPEC
"#,
    )?;

    fs::create_dir_all(workspace_root.join("docker"))?;
    fs::write(
        workspace_root.join("docker/Dockerfile"),
        "FROM alpine:3.20\n",
    )?;

    let takd = takd_bin();
    init_direct_agent(&takd, &roots, "override-remote-container-builder");
    let mut serve_env = simulated_container_runtime_env(temp.path())
        .into_iter()
        .collect::<Vec<_>>();
    serve_env.push((
        "TAKD_REMOTE_EXEC_ROOT".into(),
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
    let output = run_tak_output(
        &workspace_root,
        &["run", "--remote", "--container", "check"],
        &env,
    )?;

    assert!(output.status.success(), "status: {:?}", output.status);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(
            "warning: --container is redundant with --remote; remote execution already implies a containerized runtime"
        ),
        "stderr:\n{stderr}"
    );
    Ok(())
}
