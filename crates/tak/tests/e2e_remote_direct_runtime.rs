//! Black-box E2E contract for strict remote single-node execution.

use std::collections::BTreeMap;

use anyhow::Result;

#[allow(dead_code)]
mod support;
use support::e2e_harness::{find_free_tcp_port, run_tak_expect_success, spawn_daemon, write_tasks};

#[test]
fn e2e_remote_only_single_runs_via_real_remote_takd() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let remote_port = find_free_tcp_port()?;

    write_tasks(
        temp.path(),
        &format!(
            r#"
REMOTE = Remote(id="remote-primary", endpoint="http://127.0.0.1:{remote_port}")

SPEC = module_spec(tasks=[
  task(
    "remote_only",
    steps=[cmd("echo", "remote_only")],
    execution=RemoteOnly(REMOTE),
  )
])
SPEC
"#
        ),
    )?;

    let env = BTreeMap::new();
    let _remote_worker = spawn_daemon(
        temp.path().join("remote-worker.sock"),
        temp.path().join("remote-worker.sqlite"),
        Some(remote_port),
        &env,
    )?;
    let local_daemon = spawn_daemon(
        temp.path().join("local-daemon.sock"),
        temp.path().join("local-daemon.sqlite"),
        None,
        &env,
    )?;

    let run = run_tak_expect_success(
        temp.path(),
        &["run", "apps/web:remote_only"],
        Some(&local_daemon.socket_path),
        &env,
    )?;
    assert!(run.contains("placement=remote"));
    assert!(run.contains("remote_node=remote-primary"));

    Ok(())
}
