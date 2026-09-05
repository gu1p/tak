use takd::{AttemptCompletion, ResultAcceptance, RunStore, SchedulerNode};
use tak_proto::local_daemon::v2::RunEventKind;

use crate::support::v2_run::{ARCHIVE, scheduler::dependent_jobs};

#[test]
fn success_releases_the_slot_and_promotes_dependencies_across_restart() {
    let (temp, store, run_id) = committed("success", true);
    let nodes = [SchedulerNode::with_execution_slots("worker-a", 1)];
    let root = store.reserve_next(&nodes).unwrap().unwrap();
    assert_eq!(root.job_id, "job-0");
    assert_eq!(
        store.complete_attempt(&root, success()).unwrap(),
        ResultAcceptance::Applied
    );
    let db = temp.path().join("takd.sqlite");
    drop(store);

    let restored = RunStore::with_db_path(db).unwrap();
    assert!(restored.events_after(&run_id, 0).unwrap().iter().any(|event| {
        event.kind == RunEventKind::Queued && event.job_id.as_deref() == Some("job-1")
    }));
    let older_ready = restored.reserve_next(&nodes).unwrap().unwrap();
    assert_eq!(older_ready.job_id, "job-2");
    restored.complete_attempt(&older_ready, success()).unwrap();
    let child = restored.reserve_next(&nodes).unwrap().unwrap();
    assert_eq!(child.job_id, "job-1");
    let details = restored.get_run(&run_id).unwrap().unwrap();
    assert_eq!(details.jobs[0].state, "succeeded");
    assert_eq!(details.jobs[1].state, "transferring");
    assert_eq!(details.jobs[2].state, "succeeded");
}

#[test]
fn failed_dependencies_skip_descendants_but_keep_going_runs_unrelated_work() {
    let (_temp, store, run_id) = committed("failure", true);
    let nodes = [SchedulerNode::with_execution_slots("worker-a", 1)];
    let root = store.reserve_next(&nodes).unwrap().unwrap();
    assert_eq!(
        store.complete_attempt(&root, failed()).unwrap(),
        ResultAcceptance::Applied
    );

    let unrelated = store.reserve_next(&nodes).unwrap().unwrap();
    assert_eq!(unrelated.job_id, "job-2");
    let details = store.get_run(&run_id).unwrap().unwrap();
    assert_eq!(details.jobs[1].state, "skipped");
    assert_eq!(details.jobs[2].state, "transferring");
}

#[test]
fn fail_fast_stops_new_dispatches_after_the_first_failure() {
    let (_temp, store, run_id) = committed("fail-fast", false);
    let nodes = [SchedulerNode::with_execution_slots("worker-a", 1)];
    let root = store.reserve_next(&nodes).unwrap().unwrap();
    store.complete_attempt(&root, failed()).unwrap();

    assert!(store.reserve_next(&nodes).unwrap().is_none());
    let details = store.get_run(&run_id).unwrap().unwrap();
    assert_eq!(details.summary.state.as_str(), "failed");
    assert!(details.jobs[1..].iter().all(|job| job.state == "skipped"));
}

fn committed(key: &str, keep_going: bool) -> (tempfile::TempDir, RunStore, String) {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let request = dependent_jobs(key, keep_going);
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

fn success() -> AttemptCompletion {
    AttemptCompletion::Succeeded {
        terminal_digest: "a".repeat(64),
    }
}

fn failed() -> AttemptCompletion {
    AttemptCompletion::Failed {
        terminal_digest: "b".repeat(64),
        exit_code: None,
    }
}
