use std::sync::Arc;
use std::time::Duration;

use tak_core::v2::Step;
use tak_proto::{
    local_daemon::v2::RunEventKind,
    worker_v2::{WorkerAttemptIdentity, WorkerAttemptState},
};
use takd::{AttemptTransport, RemoteAttemptTransport, RunStore, TorBroker};

use crate::support::{v2_remote_origin, v2_run::scheduler::commit, worker_http::start_server};

#[tokio::test]
async fn remote_events_observed_after_run_cancellation_are_rejected_by_the_origin_fence() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let worker = start_server().await;
    let peers = v2_remote_origin::peers(worker.addr);
    let store = RunStore::with_db_path(temp.path().join("origin.sqlite")).unwrap();
    let mut submission = v2_remote_origin::submission();
    submission.run.tasks[0].steps = vec![Step::Cmd {
        argv: vec![
            "/bin/sh".into(),
            "-c".into(),
            "printf 'late-log\\n'; sleep 30".into(),
        ],
        cwd: None,
        env: Default::default(),
    }];
    submission.run.tasks[0].outputs.clear();
    let run_id = commit(&store, &submission, "alice");
    let command = store.reserve_next(&peers.scheduler_nodes()).unwrap().unwrap();
    let identity = WorkerAttemptIdentity {
        run_id: command.run_id.clone(),
        job_id: command.job_id.clone(),
        node_id: command.node_id.clone(),
        authored_attempt: command.authored_attempt,
        dispatch_generation: command.dispatch_generation,
        fencing_token: command.fencing_token.clone(),
    };
    let transport = Arc::new(RemoteAttemptTransport::new(
        store.clone(),
        TorBroker::new(),
        peers,
    ));
    assert!(transport.dispatch(&command).await.is_ok());
    store.ack_dispatch(&command).unwrap();
    tokio::time::timeout(Duration::from_secs(3), async {
        while worker
            .store
            .observe_worker_v2_attempt(&identity, 0)
            .unwrap()
            .events
            .is_empty()
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();

    store.cancel(&run_id).unwrap();
    let _ = transport.reconcile(&command).await;
    let late_logs = store
        .events_after(&run_id, 0)
        .unwrap()
        .into_iter()
        .filter(|event| event.kind == RunEventKind::Stdout)
        .count();
    worker.store.cancel_worker_v2_attempt(&identity).unwrap();
    tokio::time::timeout(Duration::from_secs(3), async {
        while worker
            .store
            .observe_worker_v2_attempt(&identity, 0)
            .unwrap()
            .state
            != WorkerAttemptState::Completed
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(late_logs, 0);
}
