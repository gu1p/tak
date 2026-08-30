use std::collections::BTreeMap;

use tak_proto::local_daemon::v2::WorkspaceDisposition;
use takd::{RunStore, SchedulerNode};

use crate::support::v2_run::{ARCHIVE, scheduler::independent_jobs};

#[test]
fn scheduler_round_robins_submitters_then_runs_and_survives_restart() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let store = RunStore::with_db_path(db.clone()).unwrap();
    let mut labels = BTreeMap::new();
    for (label, submitter) in [
        ("a1", "alice"),
        ("a2", "alice"),
        ("b1", "bob"),
        ("b2", "bob"),
    ] {
        let run_id = commit(&store, label, submitter);
        labels.insert(run_id, label);
    }
    let node = [SchedulerNode::with_execution_slots("worker-a", 8)];
    let mut order = (0..4)
        .map(|_| store.reserve_next(&node).unwrap().unwrap().run_id)
        .collect::<Vec<_>>();
    drop(store);

    let restored = RunStore::with_db_path(db).unwrap();
    order.extend((4..8).map(|_| restored.reserve_next(&node).unwrap().unwrap().run_id));
    let labels = order
        .iter()
        .map(|run_id| labels[run_id])
        .collect::<Vec<_>>();
    assert_eq!(labels, ["a1", "b1", "a2", "b2", "a1", "b1", "a2", "b2"]);
}

fn commit(store: &RunStore, key: &str, submitter: &str) -> String {
    let request = independent_jobs(key, 2);
    let accepted = store.submit(&request, submitter).unwrap();
    if matches!(
        accepted.workspace,
        WorkspaceDisposition::UploadRequired { .. }
    ) {
        store
            .upload_workspace(
                &accepted.run_id,
                &request.run.workspace.manifest.fingerprint,
                ARCHIVE.len() as u64,
                0,
                &ARCHIVE,
            )
            .unwrap();
    }
    store.commit(&accepted.run_id).unwrap();
    accepted.run_id
}
