use tak_core::remote_inventory::{RemoteInventory, RemoteRecord};
use tak_core::v2::{RemoteRequirements, RemoteSelection, RunSubmission};
use tak_proto::worker_v2::{WorkerResources, WorkerSnapshot};
use takd::{PeerManager, RunStore};

use crate::support::v2_run::{self, scheduler::commit};

#[test]
fn reservation_rechecks_live_inventory_requirements_after_submission() {
    let peers = PeerManager::default();
    peers.apply_inventory(inventory(true));
    peers.mark_worker_snapshot("worker-a", snapshot());
    let requirements = requirements();
    let mut request = v2_run::submission("live-inventory-filter", "secret");
    request.run.jobs[0].placement_policy.selection = RemoteSelection::Balanced;
    request.run.jobs[0].placement_candidates = peers.remote_candidates(&requirements);
    assert_eq!(request.run.jobs[0].placement_candidates.len(), 1);
    let request = with_candidate_requirements(request, &requirements);

    let temp = tempfile::tempdir().unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    commit(&store, &request, "alice");
    peers.apply_inventory(inventory(false));

    assert_eq!(peers.scheduler_nodes().len(), 1, "worker remains healthy");
    assert!(peers.remote_candidates(&requirements).is_empty());
    assert!(
        store
            .reserve_next(&peers.scheduler_nodes())
            .unwrap()
            .is_none(),
        "stale candidate must not reserve after losing pool/tag/capability eligibility"
    );
}

fn with_candidate_requirements(
    request: RunSubmission,
    requirements: &RemoteRequirements,
) -> RunSubmission {
    let mut value = serde_json::to_value(request).unwrap();
    value["run"]["jobs"][0]["placement_candidates"][0]["requirements"] =
        serde_json::to_value(requirements).unwrap();
    serde_json::from_value(value)
        .expect("protocol v2 must persist structured requirements with each remote candidate")
}

fn requirements() -> RemoteRequirements {
    RemoteRequirements {
        pool: Some("build".into()),
        required_tags: vec!["builder".into()],
        required_capabilities: vec!["linux".into()],
        transport: Some("direct".into()),
    }
}

fn inventory(eligible: bool) -> RemoteInventory {
    RemoteInventory {
        version: 1,
        remotes: vec![RemoteRecord {
            node_id: "worker-a".into(),
            display_name: "worker-a".into(),
            base_url: "http://127.0.0.1:9".into(),
            bearer_token: "secret".into(),
            pools: values(eligible, "build"),
            tags: values(eligible, "builder"),
            capabilities: values(eligible, "linux"),
            transport: "direct".into(),
            enabled: true,
        }],
    }
}

fn values(present: bool, value: &str) -> Vec<String> {
    if present { vec![value.into()] } else { vec![] }
}

fn snapshot() -> WorkerSnapshot {
    WorkerSnapshot {
        protocol_version: 2,
        node_id: "worker-a".into(),
        healthy: true,
        sampled_at_ms: 1,
        capacity: resources(8),
        usage: resources(0),
        queue_depth: 0,
        cached_content: vec![],
        processes: vec![],
    }
}

fn resources(execution_slots: u32) -> WorkerResources {
    WorkerResources {
        cpu_millis: 8_000,
        memory_bytes: 16 * 1024 * 1024 * 1024,
        execution_slots,
    }
}
