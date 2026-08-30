use std::collections::BTreeMap;

use tak_proto::local_daemon::v2::RunEventKind;
use takd::{RunStore, SchedulerNode};

use crate::support::v2_run::{ARCHIVE, scheduler::independent_jobs};

#[test]
fn ten_equal_jobs_survive_restart_with_five_dispatches_per_worker() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("takd.sqlite");
    let request = independent_jobs("round-robin", 10);
    let store = RunStore::with_db_path(db.clone()).unwrap();
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
    let nodes = [
        SchedulerNode::with_execution_slots("worker-a", 5),
        SchedulerNode::with_execution_slots("worker-b", 5),
    ];
    for _ in 0..3 {
        store.reserve_next(&nodes).unwrap().unwrap();
    }
    drop(store);

    let restored = RunStore::with_db_path(db).unwrap();
    for _ in 3..10 {
        restored.reserve_next(&nodes).unwrap().unwrap();
    }
    assert!(restored.reserve_next(&nodes).unwrap().is_none());
    let details = restored.get_run(&run.run_id).unwrap().unwrap();
    assert_eq!(details.summary.state.as_str(), "running");
    assert!(
        details
            .jobs
            .iter()
            .all(|job| job.state == "transferring" && job.attempt == 1)
    );
    let counts = details
        .jobs
        .iter()
        .fold(BTreeMap::new(), |mut counts, job| {
            *counts.entry(job.node_id.as_deref().unwrap()).or_insert(0) += 1;
            counts
        });
    assert_eq!(counts, BTreeMap::from([("worker-a", 5), ("worker-b", 5)]));
    assert_eq!(restored.pending_dispatches().unwrap().len(), 10);
    assert_eq!(
        restored
            .events_after(&run.run_id, 0)
            .unwrap()
            .iter()
            .filter(|event| event.kind == RunEventKind::Transferring)
            .count(),
        10
    );
}
