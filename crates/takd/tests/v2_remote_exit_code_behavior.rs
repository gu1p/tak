use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use tak_core::v2::{RunSubmission, Step};
use tak_proto::local_daemon::v2::RunEventKind;
use takd::{AttemptCoordinator, RemoteAttemptTransport, RunStore, TorBroker};

use crate::support::{v2_remote_origin, v2_run::scheduler::commit, worker_http::start_server};

#[tokio::test]
async fn remote_worker_exit_code_reaches_the_origin_terminal_summary_and_event() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let worker = start_server().await;
    let peers = v2_remote_origin::peers(worker.addr);
    let store = RunStore::with_db_path(temp.path().join("origin.sqlite")).unwrap();
    let run_id = commit(&store, &failing_submission(), "alice");
    store
        .reserve_next(&peers.scheduler_nodes())
        .unwrap()
        .unwrap();
    let transport = Arc::new(RemoteAttemptTransport::new(
        store.clone(),
        TorBroker::new(),
        peers,
    ));
    let mut coordinator = AttemptCoordinator::new(store.clone(), transport);
    tokio::time::timeout(Duration::from_secs(5), async {
        while !store.summary(&run_id).unwrap().unwrap().state.is_terminal() {
            coordinator.drive_once().await.unwrap();
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();

    assert_eq!(store.summary(&run_id).unwrap().unwrap().exit_code, Some(7));
    assert!(
        store
            .events_after(&run_id, 0)
            .unwrap()
            .iter()
            .any(|event| { event.kind == RunEventKind::Failed && event.exit_code == Some(7) })
    );
}

fn failing_submission() -> RunSubmission {
    let mut request = v2_remote_origin::submission();
    request.run.tasks[0].steps = vec![Step::Cmd {
        argv: vec!["/bin/sh".into(), "-c".into(), "exit 7".into()],
        cwd: None,
        env: BTreeMap::new(),
    }];
    request.run.tasks[0].outputs.clear();
    RunSubmission::new("remote-exit-seven", request.run, request.environment_values).unwrap()
}
