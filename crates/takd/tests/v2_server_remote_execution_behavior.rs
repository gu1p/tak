use std::time::Duration;

use tak_proto::local_daemon::v2::RunLifecycleState;
use takd::{RunStore, TorBroker, new_shared_manager_with_db};

use crate::support::{
    takd_bin, v2_remote_origin, v2_run::scheduler::commit, worker_http::start_server,
};

#[tokio::test]
async fn real_daemon_driver_schedules_remote_only_work_on_a_v2_worker() {
    let temp = tempfile::tempdir().unwrap();
    let worker = start_server().await;
    let db = temp.path().join("origin.sqlite");
    let socket = temp.path().join("r.sock");
    let peers = v2_remote_origin::peers(worker.addr);
    let store = RunStore::with_db_path(db.clone()).unwrap();
    let manager = new_shared_manager_with_db(db).unwrap();
    let server_store = store.clone();
    let server_socket = socket.clone();
    let remote_inventory = temp.path().join("remotes.toml");
    let server = tokio::spawn(async move {
        let (_shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        takd::run_server_with_local_attempt_executable_and_remote_inventory_until_shutdown(
            &server_socket,
            manager,
            TorBroker::new(),
            peers,
            server_store,
            takd_bin(),
            remote_inventory,
            shutdown_rx,
        )
        .await
    });
    tokio::time::timeout(Duration::from_secs(2), async {
        while !socket.exists() && !server.is_finished() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    if server.is_finished() {
        panic!("origin daemon exited before serving: {:?}", server.await);
    }
    let run_id = commit(&store, &v2_remote_origin::submission(), "alice");
    tokio::time::timeout(Duration::from_secs(30), async {
        while !store.summary(&run_id).unwrap().unwrap().state.is_terminal() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap_or_else(|error| {
        panic!(
            "remote-only run did not finish: {error:?}; summary={:?}; events={:?}; server_finished={}",
            store.summary(&run_id).unwrap(),
            store.events_after(&run_id, 0).unwrap(),
            server.is_finished(),
        )
    });
    assert_eq!(
        store.summary(&run_id).unwrap().unwrap().state,
        RunLifecycleState::Succeeded
    );
    assert_eq!(
        store.output_manifest(&run_id).unwrap().unwrap()[0].path,
        "result.txt"
    );
    server.abort();
}
