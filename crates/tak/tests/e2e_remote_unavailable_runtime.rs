//! Black-box E2E contract for strict remote unavailability errors.

use std::collections::BTreeMap;

use anyhow::Result;

#[allow(dead_code)]
mod support;
use support::e2e_harness::{find_free_tcp_port, run_tak_expect_failure, spawn_daemon, write_tasks};

#[test]
fn e2e_remote_only_single_unavailable_fails_without_local_fallback() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let unavailable_port = find_free_tcp_port()?;
    let local_marker = temp.path().join("should_not_exist_locally.txt");

    write_tasks(
        temp.path(),
        &format!(
            r#"
REMOTE = Remote(id="remote-down", endpoint="http://127.0.0.1:{unavailable_port}")

SPEC = module_spec(tasks=[
  task(
    "remote_down",
    steps=[cmd("sh", "-c", "echo should_not_run > should_not_exist_locally.txt")],
    execution=RemoteOnly(REMOTE),
  )
])
SPEC
"#
        ),
    )?;

    let env = BTreeMap::new();
    let local_daemon = spawn_daemon(
        temp.path().join("local-daemon.sock"),
        temp.path().join("local-daemon.sqlite"),
        None,
        &env,
    )?;

    let (_stdout, stderr) = run_tak_expect_failure(
        temp.path(),
        &["run", "apps/web:remote_down"],
        Some(&local_daemon.socket_path),
        &env,
    )?;
    assert!(stderr.contains("infra error"));
    assert!(stderr.contains("unavailable at"));
    assert!(stderr.contains("remote-down"));
    assert!(!local_marker.exists());

    Ok(())
}
