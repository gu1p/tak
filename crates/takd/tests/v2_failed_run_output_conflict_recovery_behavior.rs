use tak_proto::local_daemon::v2::RunLifecycleState;
use takd::{DispatchCommand, RunStore, SchedulerNode};

use crate::support::v2_run::{
    final_outputs::{seed_value, succeeded},
    output_conflicts::final_sink,
    scheduler::commit,
};

#[test]
fn terminal_republication_clears_a_conflict_from_the_failed_attempt() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let store = RunStore::with_db_path(db.clone()).unwrap();
    let request = final_sink("failed-output-conflict-recovery");
    let run_id = commit(&store, &request, "alice");
    let nodes = [SchedulerNode::with_execution_slots("local", 1)];

    let first = store.reserve_next(&nodes).unwrap().unwrap();
    let first_task = task_id(&request, &first);
    seed_value(&db, &first, first_task, "shared/value.txt", b"first");
    store.complete_attempt(&first, succeeded()).unwrap();

    let second = store.reserve_next(&nodes).unwrap().unwrap();
    let second_task = task_id(&request, &second);
    seed_value(&db, &second, second_task, "shared/value.txt", b"second");
    store.complete_attempt(&second, succeeded()).unwrap();

    assert_eq!(
        store.summary(&run_id).unwrap().unwrap().state,
        RunLifecycleState::Failed
    );
    let artifacts = store.output_manifest(&run_id).unwrap().unwrap();
    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].path, "shared/value.txt");
    let chunk = store
        .output_chunk(&artifacts[0].artifact_id, 0, 1024)
        .unwrap()
        .unwrap();
    assert_eq!(chunk.bytes, b"first");
}

fn task_id<'a>(request: &'a tak_core::v2::RunSubmission, command: &DispatchCommand) -> &'a str {
    request
        .run
        .jobs
        .iter()
        .find(|job| job.job_id == command.job_id)
        .unwrap()
        .task_ids[0]
        .as_str()
}
