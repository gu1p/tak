use std::num::NonZeroU32;

use takd::{AttemptCompletion, ResultAcceptance, RunStore, SchedulerNode};

use crate::support::v2_run::scheduler::{commit, independent_jobs};

#[test]
fn maximum_retry_backoff_does_not_strand_the_attempt_reservation() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let mut request = independent_jobs("maximum-backoff", 1);
    request.run.jobs[0].retry.max_attempts = NonZeroU32::new(2).unwrap();
    request.run.jobs[0].retry.backoff_millis = u64::MAX;
    request.run.jobs[0].retry.max_backoff_millis = 0;
    commit(&store, &request, "uid:1");
    let nodes = [SchedulerNode::with_execution_slots("worker-a", 1)];
    let command = store.reserve_next(&nodes).unwrap().unwrap();
    assert_eq!(
        store.ack_dispatch(&command).unwrap(),
        ResultAcceptance::Applied
    );

    let failed = AttemptCompletion::Failed {
        terminal_digest: "b".repeat(64),
    };
    assert_eq!(
        store.complete_attempt(&command, failed).unwrap(),
        ResultAcceptance::Applied
    );
    assert!(store.reserve_next(&nodes).unwrap().is_none());
}
