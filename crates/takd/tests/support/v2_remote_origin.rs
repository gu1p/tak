use std::collections::BTreeMap;
use std::net::SocketAddr;

use tak_core::remote_inventory::{RemoteInventory, RemoteRecord};
use tak_core::v2::{OutputSelector, PlacementCandidate, PlacementKind, RunSubmission, Step};
use tak_proto::worker_v2::{WorkerResources, WorkerSnapshot};
use takd::PeerManager;

use super::v2_run;

pub fn submission() -> RunSubmission {
    let mut request = v2_run::submission("remote-origin-restart", "secret");
    request.run.tasks[0].steps = vec![Step::Cmd {
        argv: vec![
            "/bin/sh".into(),
            "-c".into(),
            "sleep 0.05; test \"$TOKEN\" = secret; printf 'remote-log\\n'; printf 'remote-output\\n' > result.txt".into(),
        ],
        cwd: None,
        env: BTreeMap::new(),
    }];
    request.run.tasks[0].outputs = vec![OutputSelector::Path {
        value: "result.txt".into(),
    }];
    request.run.jobs[0].placement_policy.policy_id = "workers".into();
    request.run.jobs[0].placement_candidates = vec![PlacementCandidate {
        node_id: "builder-a".into(),
        kind: PlacementKind::Remote,
        transport: Some("direct".into()),
        reason: "healthy protocol-v2 worker".into(),
        tier: 0,
        requirements: None,
    }];
    RunSubmission::new(
        request.idempotency_key,
        request.run,
        request.environment_values,
    )
    .unwrap()
}

pub fn peers(address: SocketAddr) -> PeerManager {
    let peers = PeerManager::default();
    peers.apply_inventory(RemoteInventory {
        version: 1,
        remotes: vec![RemoteRecord {
            node_id: "builder-a".into(),
            display_name: "builder-a".into(),
            base_url: format!("http://{address}"),
            bearer_token: "secret".into(),
            pools: vec![],
            tags: vec![],
            capabilities: vec![],
            transport: "direct".into(),
            enabled: true,
        }],
    });
    peers.mark_worker_snapshot(
        "builder-a",
        WorkerSnapshot {
            protocol_version: 2,
            node_id: "builder-a".into(),
            healthy: true,
            sampled_at_ms: 1,
            capacity: resources(2),
            usage: resources(0),
            queue_depth: 0,
            cached_content: vec![],
            processes: vec![],
        },
    );
    peers
}

fn resources(slots: u32) -> WorkerResources {
    WorkerResources {
        cpu_millis: 8_000,
        memory_bytes: 16_000,
        execution_slots: slots,
    }
}
