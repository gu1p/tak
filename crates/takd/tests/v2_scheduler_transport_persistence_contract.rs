use takd::{ResultAcceptance, RunStore, SchedulerNode};

use crate::support::v2_run::scheduler::{commit, independent_jobs};

#[test]
fn selected_remote_transport_is_fenced_and_survives_dispatch_reconcile_and_cancel_restarts() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let mut submission = independent_jobs("transport-recovery", 1);
    submission.run.jobs[0].placement_candidates[0].transport = Some("tor".into());
    let store = RunStore::with_db_path(db.clone()).unwrap();
    let run_id = commit(&store, &submission, "uid:1");
    let command = store
        .reserve_next(&[
            SchedulerNode::with_execution_slots("worker-a", 1).with_transport("tor")
        ])
        .unwrap()
        .unwrap();
    assert_eq!(command.transport.as_deref(), Some("tor"));
    let mut wrong = command.clone();
    wrong.transport = Some("direct".into());
    assert_eq!(store.ack_dispatch(&wrong).unwrap(), ResultAcceptance::Stale);
    drop(store);

    let store = RunStore::with_db_path(db.clone()).unwrap();
    assert_eq!(
        store.pending_dispatches().unwrap(),
        std::slice::from_ref(&command)
    );
    assert_eq!(store.ack_dispatch(&command).unwrap(), ResultAcceptance::Applied);
    drop(store);

    let store = RunStore::with_db_path(db.clone()).unwrap();
    assert_eq!(
        store.running_attempts_for_reconciliation().unwrap(),
        std::slice::from_ref(&command)
    );
    store.cancel(&run_id).unwrap();
    drop(store);

    let store = RunStore::with_db_path(db).unwrap();
    assert_eq!(store.pending_cancellations().unwrap(), [command]);
}
