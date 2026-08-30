use std::num::NonZeroU32;

use takd::{
    AttemptCompletion, ResultAcceptance, RunStore, SchedulerNode, UnknownOutcomeResolution,
};

use crate::support::v2_run::{ARCHIVE, scheduler::independent_jobs};

#[test]
fn an_idempotent_unknown_outcome_retries_and_fences_the_old_attempt() {
    let (_temp, store, run_id) = committed("retry", true);
    let nodes = [SchedulerNode::with_execution_slots("worker-a", 1)];
    let first = store.reserve_next(&nodes).unwrap().unwrap();
    assert_eq!(
        store.ack_dispatch(&first).unwrap(),
        ResultAcceptance::Applied
    );
    assert_eq!(
        store.resolve_unknown_attempt(&first).unwrap(),
        UnknownOutcomeResolution::Retrying
    );
    let second = store.reserve_next(&nodes).unwrap().unwrap();
    assert_eq!(second.authored_attempt, 2);
    assert_ne!(first.fencing_token, second.fencing_token);
    assert_eq!(
        store.complete_attempt(&first, success("a")).unwrap(),
        ResultAcceptance::Stale
    );
    assert_eq!(
        store.complete_attempt(&second, success("b")).unwrap(),
        ResultAcceptance::Applied
    );
    assert_eq!(
        store
            .get_run(&run_id)
            .unwrap()
            .unwrap()
            .summary
            .state
            .as_str(),
        "succeeded"
    );
}

#[test]
fn a_non_idempotent_unknown_outcome_fails_without_retrying() {
    let (_temp, store, run_id) = committed("no-retry", false);
    let nodes = [SchedulerNode::with_execution_slots("worker-a", 1)];
    let first = store.reserve_next(&nodes).unwrap().unwrap();
    store.ack_dispatch(&first).unwrap();
    assert_eq!(
        store.resolve_unknown_attempt(&first).unwrap(),
        UnknownOutcomeResolution::Failed
    );
    assert!(store.reserve_next(&nodes).unwrap().is_none());
    assert_eq!(
        store
            .get_run(&run_id)
            .unwrap()
            .unwrap()
            .summary
            .state
            .as_str(),
        "failed"
    );
}

fn committed(key: &str, idempotent: bool) -> (tempfile::TempDir, RunStore, String) {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let mut request = independent_jobs(key, 1);
    request.run.tasks[0].idempotent = idempotent;
    request.run.jobs[0].idempotent = idempotent;
    request.run.jobs[0].retry.max_attempts = NonZeroU32::new(2).unwrap();
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

fn success(seed: &str) -> AttemptCompletion {
    AttemptCompletion::Succeeded {
        terminal_digest: seed.repeat(64),
    }
}
