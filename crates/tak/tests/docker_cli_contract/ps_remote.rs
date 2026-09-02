#![cfg(unix)]

use std::collections::BTreeMap;

use anyhow::Result;
use base64::{Engine as _, engine::general_purpose::STANDARD};

use super::ps_status_payload::{active_job, node_status_payload};
use crate::support::{
    remote_daemon_v2::{FakeRemoteDaemon, remote},
    run_tak_output,
};

#[test]
fn docker_ps_uses_daemon_inventory_selectors_and_status_detail() -> Result<()> {
    let root = tempfile::tempdir()?;
    let detail = node_status_payload(
        "builder-a",
        "http://builder-a.onion",
        vec![
            active_job(
                "//:docker-run",
                "docker-run-1",
                "docker-run",
                "image:alpine:3.20",
                "sleep 30",
            ),
            active_job(
                "//apps/web:build",
                "task-run-1",
                "task",
                "dockerfile:docker/Dockerfile",
                "make build",
            ),
        ],
    );
    let daemon = FakeRemoteDaemon::spawn(
        root.path(),
        vec![
            serde_json::json!({"type": "RemoteList", "remotes": [remote("builder-a")]}),
            serde_json::json!({"type": "RemoteStatus", "remotes": [{
                "remote": remote("builder-a"),
                "snapshot": {"protocol_version": 2, "node_id": "builder-a", "healthy": true,
                    "sampled_at_ms": 1, "capacity": {"cpu_millis": 8000, "memory_bytes": 16000,
                    "execution_slots": 8}, "usage": {"cpu_millis": 1000, "memory_bytes": 4000,
                    "execution_slots": 2}, "queue_depth": 1, "cached_content": [], "processes": []},
                "detail_base64": STANDARD.encode(detail), "error": null, "peer": null
            }]}),
        ],
    );
    let env = BTreeMap::from([("TAKD_SOCKET".into(), daemon.socket().display().to_string())]);
    let output = run_tak_output(
        root.path(),
        &[
            "--pool",
            "build",
            "--tag",
            "linux",
            "--capability",
            "docker",
            "--arch",
            "arm64",
            "--os",
            "linux",
            "--transport",
            "tor",
            "docker",
            "ps",
        ],
        &env,
    )?;

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("node=builder-a") && stdout.contains("kind=docker-run"),
        "{stdout}"
    );
    assert!(stdout.contains("source=image:alpine:3.20") && stdout.contains("command=sleep 30"));
    let requests = daemon.finish();
    assert_eq!(requests[0]["operation"]["type"], "ListRemotes");
    assert_eq!(requests[1]["operation"]["type"], "GetRemoteStatus");
    assert_eq!(
        requests[1]["operation"]["node_ids"],
        serde_json::json!(["builder-a"])
    );
    Ok(())
}
