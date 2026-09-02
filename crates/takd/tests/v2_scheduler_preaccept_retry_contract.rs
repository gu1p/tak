use std::num::NonZeroU32;

use takd::{
    AttemptCompletion, ResultAcceptance, RunStore, SchedulerNode, UnknownOutcomeResolution,
};

use crate::support::v2_run::scheduler::{commit, independent_jobs};

#[test]
fn a_non_idempotent_attempt_lost_before_acceptance_retries_within_budget() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let mut request = independent_jobs("pre-accept-retry", 1);
    request.run.tasks[0].idempotent = false;
    request.run.jobs[0].idempotent = false;
    request.run.jobs[0].retry.max_attempts = NonZeroU32::new(2).unwrap();
    let run_id = commit(&store, &request, "uid:1");
    let nodes = [SchedulerNode::with_execution_slots("worker-a", 1)];
    let first = store.reserve_next(&nodes).unwrap().unwrap();

    assert_eq!(
        store.resolve_unknown_attempt(&first).unwrap(),
        UnknownOutcomeResolution::Retrying
    );
    let second = store.reserve_next(&nodes).unwrap().unwrap();
    assert_eq!(second.authored_attempt, 2);
    assert_ne!(first.fencing_token, second.fencing_token);
    assert_eq!(
        store
            .complete_attempt(
                &second,
                AttemptCompletion::Succeeded {
                    terminal_digest: "c".repeat(64),
                },
            )
            .unwrap(),
        ResultAcceptance::Applied
    );
    assert_eq!(
        store.get_run(&run_id).unwrap().unwrap().summary.state.as_str(),
        "succeeded"
    );
}
