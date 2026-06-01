#![cfg(test)]

use crate::cli::remote_inventory::RemoteRecord;
use crate::cli::remote_status::{DaemonPeerSnapshot, RemoteStatusResult};

use super::render_snapshot;

#[test]
fn daemon_degraded_peer_renders_degraded_status() {
    let output = render_snapshot(&[RemoteStatusResult {
        remote: remote(),
        status: None,
        error: None,
        peer: Some(degraded_peer()),
    }]);

    assert!(output.contains("builder-a transport=tor state=degraded"));
    assert!(output.contains("status=degraded detail=peer health is recovering"));
    assert!(!output.contains("status=ok detail=peer health is recovering"));
}

fn remote() -> RemoteRecord {
    RemoteRecord {
        node_id: "builder-a".to_string(),
        display_name: "Builder A".to_string(),
        base_url: "http://builder-a.onion".to_string(),
        bearer_token: String::new(),
        pools: vec!["build".to_string()],
        tags: vec!["linux".to_string()],
        capabilities: vec!["docker".to_string()],
        transport: "tor".to_string(),
        enabled: true,
    }
}

fn degraded_peer() -> DaemonPeerSnapshot {
    DaemonPeerSnapshot {
        node_id: "builder-a".to_string(),
        display_name: "Builder A".to_string(),
        transport: "tor".to_string(),
        endpoint: "http://builder-a.onion".to_string(),
        state: "degraded".to_string(),
        last_heartbeat_ms: None,
        last_successful_connection_ms: None,
        last_error_summary: Some("peer health is recovering".to_string()),
        active_job_count: Some(0),
        queue_depth: Some(0),
        resource_summary: Some("cpu_available=16.00".to_string()),
        protocol_version: Some("v1".to_string()),
        heartbeat_rtt_ms: Some(1586),
        reconnect_attempts: 89,
        pools: vec!["build".to_string()],
        tags: vec!["linux".to_string()],
        capabilities: vec!["docker".to_string()],
    }
}
