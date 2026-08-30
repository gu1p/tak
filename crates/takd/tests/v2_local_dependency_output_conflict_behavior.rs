use std::time::Duration;

use tak_proto::local_daemon::v2::{RunEventKind, RunLifecycleState};
use takd::RunStore;

use crate::support::{protocol_server::spawn_protocol_server, v2_run};

#[tokio::test]
async fn dependency_output_conflict_fails_once_without_dispatching_the_consumer() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let socket = std::path::PathBuf::from(".tmp")
        .join(temp.path().file_name().unwrap())
        .join("d.sock");
    let server = spawn_protocol_server(db.clone(), socket.clone());
    wait_for(|| socket.exists()).await;
    let store = RunStore::with_db_path(db).unwrap();
    let request = v2_run::output_conflicts::dependency("dependency-conflict");
    let run_id = v2_run::scheduler::commit(&store, &request, "alice");
    wait_for(|| store.summary(&run_id).unwrap().unwrap().state.is_terminal()).await;

    assert_eq!(
        store.summary(&run_id).unwrap().unwrap().state,
        RunLifecycleState::Failed
    );
    let consumer = store
        .get_run(&run_id)
        .unwrap()
        .unwrap()
        .jobs
        .into_iter()
        .find(|job| job.job_id == "job-consume")
        .unwrap();
    assert_eq!((consumer.state.as_str(), consumer.attempt), ("failed", 1));
    assert!(store.pending_dispatches().unwrap().is_empty());
    let conflicts = store
        .events_after(&run_id, 0)
        .unwrap()
        .into_iter()
        .filter(|event| {
            event.kind == RunEventKind::Failed
                && event.message.contains("independent producers conflict")
        })
        .count();
    assert_eq!(conflicts, 1);
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
