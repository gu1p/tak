use takd::{AttemptCompletion, ResultAcceptance, RunStore, SchedulerNode};

use crate::support::v2_run::scheduler::{commit, independent_jobs};

#[test]
fn terminal_before_dispatch_ack_settles_the_durable_dispatch() {
    let (temp, store) = store();
    let run_id = commit(&store, &independent_jobs("terminal-before-ack", 1), "uid:1");
    let command = store
        .reserve_next(&[SchedulerNode::with_execution_slots("worker-a", 1)])
        .unwrap()
        .unwrap();

    let pending = store.pending_dispatches().unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0], command);
    assert_eq!(
        store.complete_attempt(&command, success()).unwrap(),
        ResultAcceptance::Applied
    );
    assert!(store.pending_dispatches().unwrap().is_empty());
    drop(store);
    assert!(
        RunStore::with_db_path(temp.path().join("takd.sqlite"))
            .unwrap()
            .pending_dispatches()
            .unwrap()
            .is_empty()
    );
    assert_eq!(run_id, command.run_id);
}

#[test]
fn attempt_identity_includes_the_selected_node() {
    let (_temp, store) = store();
    commit(&store, &independent_jobs("wrong-node", 1), "uid:1");
    let mut command = store
        .reserve_next(&[SchedulerNode::with_execution_slots("worker-a", 1)])
        .unwrap()
        .unwrap();
    command.node_id = "worker-b".into();

    assert_eq!(
        store.ack_dispatch(&command).unwrap(),
        ResultAcceptance::Stale
    );
    assert_eq!(
        store.complete_attempt(&command, success()).unwrap(),
        ResultAcceptance::Stale
    );
}

fn store() -> (tempfile::TempDir, RunStore) {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    (temp, store)
}

fn success() -> AttemptCompletion {
    AttemptCompletion::Succeeded {
        terminal_digest: "a".repeat(64),
    }
}
