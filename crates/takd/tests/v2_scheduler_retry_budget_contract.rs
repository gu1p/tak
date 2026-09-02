use std::num::NonZeroU32;

use takd::{ResultAcceptance, RunStore, SchedulerNode, UnknownOutcomeResolution};

use crate::support::v2_run::{ARCHIVE, scheduler::independent_jobs};

#[test]
fn an_idempotent_unknown_outcome_fails_after_exhausting_the_retry_budget() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let mut request = independent_jobs("retry-budget", 1);
    request.run.tasks[0].idempotent = true;
    request.run.jobs[0].idempotent = true;
    request.run.jobs[0].retry.max_attempts = NonZeroU32::new(2).unwrap();
    let submitted = store.submit(&request, "uid:1").unwrap();
    store
        .upload_workspace(
            &submitted.run_id,
            &request.run.workspace.manifest.fingerprint,
            ARCHIVE.len() as u64,
            0,
            &ARCHIVE,
        )
        .unwrap();
    store.commit(&submitted.run_id).unwrap();

    let nodes = [SchedulerNode::with_execution_slots("worker-a", 1)];
    let first = store.reserve_next(&nodes).unwrap().unwrap();
    assert_eq!(store.ack_dispatch(&first).unwrap(), ResultAcceptance::Applied);
    assert_eq!(
        store.resolve_unknown_attempt(&first).unwrap(),
        UnknownOutcomeResolution::Retrying
    );
    let second = store.reserve_next(&nodes).unwrap().unwrap();
    assert_eq!(second.authored_attempt, 2);
    assert_eq!(store.ack_dispatch(&second).unwrap(), ResultAcceptance::Applied);
    assert_eq!(
        store.resolve_unknown_attempt(&second).unwrap(),
        UnknownOutcomeResolution::Failed
    );
    assert!(store.reserve_next(&nodes).unwrap().is_none());
    assert_eq!(
        store.summary(&submitted.run_id).unwrap().unwrap().state.as_str(),
        "failed"
    );
}
