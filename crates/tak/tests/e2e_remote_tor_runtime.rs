//! Black-box E2E contract for Tor transport parity via real daemons.

use std::collections::BTreeMap;

use anyhow::Result;

#[allow(dead_code)]
mod support;
use support::e2e_harness::{find_free_tcp_port, run_tak_expect_success, spawn_daemon, write_tasks};

#[test]
fn e2e_remote_only_tor_transport_reaches_real_takd_worker() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let remote_port = find_free_tcp_port()?;
    let onion_host = "abcdefghijklmnopqrstuvwxyz234567abcdefghijklmnopqrstuvwxyz2345.onion";

    write_tasks(
        temp.path(),
        &format!(
            r#"
REMOTE = Remote(
  id="remote-tor",
  transport=RemoteTransportMode.TorOnionService(endpoint="http://{onion_host}:{remote_port}"),
)

SPEC = module_spec(tasks=[
  task(
    "remote_tor",
    steps=[cmd("sh", "-c", "true")],
    execution=RemoteOnly(REMOTE),
  )
])
SPEC
"#
        ),
    )?;

    let mut env = BTreeMap::new();
    env.insert(
        "TAK_TEST_TOR_ONION_DIAL_ADDR".to_string(),
        format!("127.0.0.1:{remote_port}"),
    );
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
        &["run", "apps/web:remote_tor"],
        Some(&local_daemon.socket_path),
        &env,
    )?;
    assert!(run.contains("placement=remote"));
    assert!(run.contains("remote_node=remote-tor"));

    Ok(())
}
