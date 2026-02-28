//! Black-box E2E contract for ordered remote fallback.

use std::collections::BTreeMap;

use anyhow::Result;

#[allow(dead_code)]
mod support;
use support::e2e_harness::{find_free_tcp_port, run_tak_expect_success, spawn_daemon, write_tasks};

#[test]
fn e2e_remote_only_list_falls_back_to_first_reachable_node() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let down_port = find_free_tcp_port()?;
    let healthy_port = find_free_tcp_port()?;

    write_tasks(
        temp.path(),
        &format!(
            r#"
REMOTE_A = Remote(id="remote-down", endpoint="http://127.0.0.1:{down_port}")
REMOTE_B = Remote(id="remote-up", endpoint="http://127.0.0.1:{healthy_port}")

SPEC = module_spec(tasks=[
  task(
    "remote_list",
    steps=[cmd("echo", "remote_list")],
    execution=RemoteOnly([REMOTE_A, REMOTE_B]),
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
        Some(healthy_port),
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
        &["run", "apps/web:remote_list"],
        Some(&local_daemon.socket_path),
        &env,
    )?;
    assert!(run.contains("placement=remote"));
    assert!(run.contains("remote_node=remote-up"));

    Ok(())
}
