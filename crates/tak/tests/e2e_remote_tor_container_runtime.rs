//! Optional black-box E2E contract for Tor + real containerized remote execution.

use std::collections::BTreeMap;
use std::fs;

use anyhow::Result;

#[allow(dead_code)]
mod support;
use support::e2e_harness::{
    detect_podman_socket, find_free_tcp_port, run_tak_expect_success, spawn_daemon, write_tasks,
};

#[test]
fn e2e_remote_tor_container_runtime_runs_inside_real_container() -> Result<()> {
    if std::env::var("TAK_E2E_REAL_CONTAINER").ok().as_deref() != Some("1") {
        eprintln!("skipping: set TAK_E2E_REAL_CONTAINER=1 to run Tor+container E2E contract");
        return Ok(());
    }

    let temp = tempfile::tempdir()?;
    let remote_port = find_free_tcp_port()?;
    let onion_host = "abcdefghijklmnopqrstuvwxyz234567abcdefghijklmnopqrstuvwxyz2345.onion";
    let image = std::env::var("TAK_E2E_REAL_CONTAINER_IMAGE")
        .unwrap_or_else(|_| "busybox:1.36".to_string());
    let marker = temp.path().join("out/container-proof.txt");

    write_tasks(
        temp.path(),
        &format!(
            r#"
REMOTE = Remote(
  id="remote-tor-container",
  transport=RemoteTransportMode.TorOnionService(endpoint="http://{onion_host}:{remote_port}"),
  runtime=ContainerRuntime(image="{image}"),
)

SPEC = module_spec(tasks=[
  task(
    "remote_tor_container",
    steps=[cmd("sh", "-c", "if [ -r /proc/1/comm ] && [ \"$(cat /proc/1/comm 2>/dev/null)\" = \"sh\" ]; then mkdir -p out && echo containerized > out/container-proof.txt; else mkdir -p out && echo host > out/container-proof.txt; exit 17; fi")],
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
    if std::env::var("TAK_PODMAN_SOCKET").is_err()
        && let Some(socket) = detect_podman_socket()
    {
        env.insert("TAK_PODMAN_SOCKET".to_string(), socket);
    }
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
        &["run", "apps/web:remote_tor_container"],
        Some(&local_daemon.socket_path),
        &env,
    )?;
    assert!(run.contains("placement=remote"));
    assert!(run.contains("remote_node=remote-tor-container"));
    assert!(run.contains("transport=tor"));
    assert!(run.contains("runtime=containerized"));

    assert_eq!(fs::read_to_string(marker)?.trim(), "containerized");

    Ok(())
}
