use std::num::NonZeroU32;

use takd::{AttemptCompletion, RunStore, SchedulerNode};

use crate::support::v2_run::scheduler::{commit, independent_jobs};

#[test]
fn retry_exit_filters_are_enforced_by_the_daemon() {
    let (_unmatched_temp, unmatched_store, unmatched_run) = committed("unmatched");
    let node = [SchedulerNode::with_execution_slots("worker-a", 1)];
    let attempt = unmatched_store.reserve_next(&node).unwrap().unwrap();
    unmatched_store
        .complete_attempt(&attempt, failure(2))
        .unwrap();
    assert!(unmatched_store.reserve_next(&node).unwrap().is_none());
    assert_eq!(
        unmatched_store
            .get_run(&unmatched_run)
            .unwrap()
            .unwrap()
            .summary
            .state
            .as_str(),
        "failed"
    );

    let (_matched_temp, matched_store, _) = committed("matched");
    let attempt = matched_store.reserve_next(&node).unwrap().unwrap();
    matched_store
        .complete_attempt(&attempt, failure(7))
        .unwrap();
    assert_eq!(
        matched_store
            .reserve_next(&node)
            .unwrap()
            .unwrap()
            .authored_attempt,
        2
    );
}

fn committed(key: &str) -> (tempfile::TempDir, RunStore, String) {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let mut request = independent_jobs(key, 1);
    request.run.jobs[0].retry.max_attempts = NonZeroU32::new(2).unwrap();
    request.run.jobs[0].retry.on_exit = vec![7];
    let run_id = commit(&store, &request, "uid:1");
    (temp, store, run_id)
}

fn failure(exit_code: i32) -> AttemptCompletion {
    AttemptCompletion::Failed {
        terminal_digest: format!("{exit_code:064x}"),
        exit_code: Some(exit_code),
    }
}
