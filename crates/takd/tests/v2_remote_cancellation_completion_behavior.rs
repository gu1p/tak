use std::sync::Arc;
use std::time::Duration;

use tak_core::v2::Step;
use tak_proto::{
    local_daemon::v2::RunLifecycleState,
    worker_v2::{WorkerAttemptIdentity, WorkerTerminalOutcome},
};
use takd::{AttemptCoordinator, RemoteAttemptTransport, RunStore, TorBroker};

use crate::support::{v2_remote_origin, v2_run::scheduler::commit, worker_http::start_server};

#[tokio::test]
async fn remote_run_stays_cancelling_until_the_worker_attempt_is_terminal() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let worker = start_server().await;
    let peers = v2_remote_origin::peers(worker.addr);
    let store = RunStore::with_db_path(temp.path().join("origin.sqlite")).unwrap();
    let mut submission = v2_remote_origin::submission();
    submission.run.tasks[0].steps = vec![Step::Cmd {
        argv: vec!["/bin/sh".into(), "-c".into(), "sleep 30".into()],
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
    let mut coordinator = AttemptCoordinator::new(store.clone(), transport);
    assert_eq!(coordinator.drive_once().await.unwrap().dispatched, 1);
    store.cancel(&run_id).unwrap();

    coordinator.drive_once().await.unwrap();
    assert_eq!(
        store.summary(&run_id).unwrap().unwrap().state,
        RunLifecycleState::Cancelling
    );
    tokio::time::timeout(Duration::from_secs(5), async {
        while store.summary(&run_id).unwrap().unwrap().state == RunLifecycleState::Cancelling {
            coordinator.drive_once().await.unwrap();
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        store.summary(&run_id).unwrap().unwrap().state,
        RunLifecycleState::Cancelled
    );
    assert_eq!(
        worker
            .store
            .observe_worker_v2_attempt(&identity, 0)
            .unwrap()
            .terminal
            .unwrap()
            .outcome,
        WorkerTerminalOutcome::Cancelled
    );
}
