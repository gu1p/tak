use std::time::Duration;

use tak_core::v2::{ContainerSource, TaskRuntime};
use tak_proto::local_daemon::v2::{RunEventKind, RunLifecycleState};
use takd::RunStore;

use crate::support::{
    env::{EnvGuard, env_lock},
    protocol_server::spawn_protocol_server,
    v2_run,
};

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn final_output_conflict_is_a_persisted_terminal_failure_without_retry() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    env.set("TAK_TEST_HOST_PLATFORM", "other");
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let socket = std::path::PathBuf::from(".tmp")
        .join(temp.path().file_name().unwrap())
        .join("d.sock");
    let server = spawn_protocol_server(db.clone(), socket.clone());
    wait_for(|| socket.exists()).await;
    let store = RunStore::with_db_path(db.clone()).unwrap();
    let mut request = v2_run::output_conflicts::final_sink("final-conflict");
    for task in &mut request.run.tasks {
        task.runtime = Some(TaskRuntime::container(ContainerSource::Image {
            image: "alpine:3.20".into(),
        }));
    }
    let run_id = v2_run::scheduler::commit(&store, &request, "alice");
    wait_for(|| store.summary(&run_id).unwrap().unwrap().state.is_terminal()).await;

    assert_eq!(
        store.summary(&run_id).unwrap().unwrap().state,
        RunLifecycleState::Failed
    );
    let jobs = store.get_run(&run_id).unwrap().unwrap().jobs;
    assert_eq!(jobs.iter().filter(|job| job.state == "failed").count(), 1);
    assert_eq!(
        jobs.iter().filter(|job| job.state == "succeeded").count(),
        1
    );
    assert!(
        jobs.iter().all(|job| job.attempt == 1),
        "unexpected job attempts: {jobs:?}"
    );
    assert!(store.pending_dispatches().unwrap().is_empty());
    drop(store);
    let reopened = RunStore::with_db_path(db).unwrap();
    let conflicts = reopened
        .events_after(&run_id, 0)
        .unwrap()
        .into_iter()
        .filter(|event| {
            event.kind == RunEventKind::Failed && event.message.contains("before `final run`")
        })
        .collect::<Vec<_>>();
    assert_eq!(conflicts.len(), 1);
    assert!(conflicts[0].message.contains("runtime=containerized"));
    assert!(conflicts[0].message.contains("runtime_engine=docker"));
    server.abort();
}

async fn wait_for(predicate: impl Fn() -> bool) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !predicate() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
}
