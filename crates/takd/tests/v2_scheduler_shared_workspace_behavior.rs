use tak_core::v2::{Affinity, Session, SessionReuse};
use takd::{AttemptCompletion, RunStore, SchedulerNode};

use crate::support::v2_run::scheduler::{commit, independent_jobs};

#[path = "v2_scheduler_shared_workspace_behavior/context.rs"]
mod context;

#[test]
fn shared_workspace_cap_and_home_survive_restart_until_release() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let store = RunStore::with_db_path(db.clone()).unwrap();
    let request = shared_run("shared-cap", 1, "shared");
    commit(&store, &request, "alice");
    let nodes = [
        SchedulerNode::with_execution_slots("worker-a", 2),
        SchedulerNode::with_execution_slots("worker-b", 2),
    ];
    let first = store.reserve_next(&nodes).unwrap().unwrap();
    assert!(store.reserve_next(&nodes).unwrap().is_none());
    drop(store);

    let restored = RunStore::with_db_path(db).unwrap();
    assert!(restored.reserve_next(&nodes).unwrap().is_none());
    restored
        .complete_attempt(
            &first,
            AttemptCompletion::Succeeded {
                terminal_digest: "5".repeat(64),
            },
        )
        .unwrap();
    let second = restored.reserve_next(&nodes).unwrap().unwrap();
    assert_eq!(second.node_id, first.node_id);
}

#[test]
fn distinct_shared_sessions_have_independent_parallelism_caps() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let mut request = shared_run("shared-distinct", 1, "one");
    let second = Session::new(
        "two",
        SessionReuse::shared_workspace(1).unwrap(),
        Some(Affinity::require_same_node("shared").unwrap()),
    )
    .unwrap();
    request.run.jobs[1].session = Some(second);
    commit(&store, &request, "alice");
    let nodes = [SchedulerNode::with_execution_slots("worker-a", 2)];
    assert!(store.reserve_next(&nodes).unwrap().is_some());
    assert!(store.reserve_next(&nodes).unwrap().is_some());
}

#[test]
fn shared_workspace_honors_capacities_greater_than_one() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let mut request = shared_run("shared-two", 2, "shared");
    let mut task = request.run.tasks[0].clone();
    task.task_id = "//:third".into();
    task.job_id = "job-2".into();
    let mut job = request.run.jobs[0].clone();
    job.job_id = task.job_id.clone();
    job.task_ids = vec![task.task_id.clone()];
    request.run.targets.push(task.task_id.clone());
    request.run.tasks.push(task);
    request.run.jobs.push(job);
    request.run.options.max_parallel_jobs = std::num::NonZeroU32::new(3).unwrap();
    commit(&store, &request, "alice");
    let nodes = [SchedulerNode::with_execution_slots("worker-a", 3)];
    assert!(store.reserve_next(&nodes).unwrap().is_some());
    assert!(store.reserve_next(&nodes).unwrap().is_some());
    assert!(store.reserve_next(&nodes).unwrap().is_none());
}

fn shared_run(key: &str, cap: u32, session_id: &str) -> tak_core::v2::RunSubmission {
    let mut request = independent_jobs(key, 2);
    let hard = Affinity::require_same_node("shared").unwrap();
    let session = Session::new(
        session_id,
        SessionReuse::shared_workspace(cap).unwrap(),
        Some(hard.clone()),
    )
    .unwrap();
    for task in &mut request.run.tasks {
        task.affinity = Some(hard.clone());
    }
    for job in &mut request.run.jobs {
        job.affinity = Some(hard.clone());
        job.session = Some(session.clone());
    }
    request
}
