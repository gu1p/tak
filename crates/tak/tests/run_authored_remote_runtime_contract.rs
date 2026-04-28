//! Black-box contract for authored `Execution.Remote(...)` runtime resolution.

use crate::support;

use std::fs;

use anyhow::Result;

use support::direct_remote_runtime::{client_env, start_direct_agent};
use support::{run_tak_expect_success, run_tak_output, write_tasks};

#[test]
fn authored_remote_only_inherits_module_default_container_runtime() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace_root = temp.path().join("workspace");
    let _agent = start_direct_agent(temp.path(), &workspace_root, "authored-remote-default");
    write_tasks(
        &workspace_root,
        r#"REMOTE = Execution.Remote(pool="build", required_tags=["builder"], required_capabilities=["linux"], transport=Transport.DirectHttps())
SPEC = module_spec(defaults=Defaults(container_runtime=Runtime.Image("alpine:3.20")), tasks=[task("check", outputs=[path("out")], steps=[cmd("sh", "-c", "mkdir -p out && printf '%s\n' \"$TAK_RUNTIME_SOURCE\" > out/runtime-source.txt")], execution=REMOTE)])
SPEC
"#,
    )?;

    let stdout =
        run_tak_expect_success(&workspace_root, &["run", "check"], &client_env(temp.path()))?;
    assert!(stdout.contains("placement=remote"), "stdout:\n{stdout}");
    assert_eq!(
        fs::read_to_string(workspace_root.join("out/runtime-source.txt"))?.trim(),
        "image"
    );
    Ok(())
}

#[test]
fn authored_remote_only_without_runtime_fails_closed() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace_root = temp.path().join("workspace");
    let _agent = start_direct_agent(temp.path(), &workspace_root, "authored-remote-missing");
    write_tasks(
        &workspace_root,
        r#"REMOTE = Execution.Remote(pool="build", required_tags=["builder"], required_capabilities=["linux"], transport=Transport.DirectHttps())
SPEC = module_spec(tasks=[task("check", steps=[cmd("sh", "-c", "echo should-not-run")], execution=REMOTE)])
SPEC
"#,
    )?;

    let output = run_tak_output(&workspace_root, &["run", "check"], &client_env(temp.path()))?;
    assert!(
        !output.status.success(),
        "status unexpectedly succeeded: {:?}",
        output.status
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(
            "task //:check requires a containerized runtime for remote execution; provide Execution.Remote(..., runtime=Runtime.Image(...)), Decision.remote(..., runtime=Runtime.Image(...)), or TASKS.py defaults.container_runtime"
        ),
        "stderr:\n{stderr}"
    );
    Ok(())
}
