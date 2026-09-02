use std::collections::BTreeMap;

use tak_core::v2::RemoteSelection;
use takd::{RunStore, SchedulerNode};

use crate::support::v2_run::{ARCHIVE, scheduler::independent_jobs};

#[path = "v2_scheduler_balanced_behavior/locality.rs"]
mod locality;
#[test]
fn balanced_equal_jobs_stay_within_one_assignment() {
    let (_temp, store, run_id) = committed_balanced_run("balanced", 10);
    let nodes = [
        SchedulerNode::with_execution_slots("worker-a", u32::MAX),
        SchedulerNode::with_execution_slots("worker-b", u32::MAX),
    ];
    for _ in 0..10 {
        store.reserve_next(&nodes).unwrap().unwrap();
        let counts = placement_counts(&store, &run_id);
        assert!((counts["worker-a"] - counts["worker-b"]).abs() <= 1);
    }
    assert_eq!(
        placement_counts(&store, &run_id),
        BTreeMap::from([("worker-a".into(), 5), ("worker-b".into(), 5)])
    );
}

#[test]
fn balanced_uses_projected_queue_pressure_when_another_resource_masks_slots() {
    let (_temp, store, run_id) = committed_balanced_run("queue", 10);
    let mut a = SchedulerNode::with_execution_slots("worker-a", 100);
    let mut b = SchedulerNode::with_execution_slots("worker-b", 100);
    for node in [&mut a, &mut b] {
        node.cpu_capacity_millis = 100;
        node.cpu_used_millis = 90;
    }
    for _ in 0..10 {
        store
            .reserve_next(&[a.clone(), b.clone()])
            .unwrap()
            .unwrap();
    }
    assert_eq!(
        placement_counts(&store, &run_id),
        BTreeMap::from([("worker-a".into(), 5), ("worker-b".into(), 5)])
    );
}

#[test]
fn balanced_prefers_an_idle_node_over_a_saturated_cached_peer() {
    let (_temp, store, run_id) = committed_balanced_run("idle", 1);
    let fingerprint = store.workspace_fingerprint(&run_id).unwrap().unwrap();
    let nodes = [
        SchedulerNode::with_execution_slots("worker-a", 10)
            .with_execution_usage(9)
            .with_cached_content(fingerprint),
        SchedulerNode::with_execution_slots("worker-b", 10),
    ];
    let dispatch = store.reserve_next(&nodes).unwrap().unwrap();
    assert_eq!(dispatch.node_id, "worker-b");
}

fn committed_balanced_run(key: &str, count: usize) -> (tempfile::TempDir, RunStore, String) {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let mut request = independent_jobs(key, count);
    for job in &mut request.run.jobs {
        job.placement_policy.selection = RemoteSelection::Balanced;
    }
    let run = store.submit(&request, "uid:1").unwrap();
    store
        .upload_workspace(
            &run.run_id,
            &request.run.workspace.manifest.fingerprint,
            ARCHIVE.len() as u64,
            0,
            &ARCHIVE,
        )
        .unwrap();
    store.commit(&run.run_id).unwrap();
    (temp, store, run.run_id)
}

fn placement_counts(store: &RunStore, run_id: &str) -> BTreeMap<String, i32> {
    store
        .get_run(run_id)
        .unwrap()
        .unwrap()
        .jobs
        .iter()
        .filter_map(|job| job.node_id.as_deref())
        .fold(
            BTreeMap::from([("worker-a".into(), 0), ("worker-b".into(), 0)]),
            |mut counts, node| {
                *counts.get_mut(node).unwrap() += 1;
                counts
            },
        )
}
