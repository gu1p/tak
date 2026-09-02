use std::time::Duration;

use takd::{NodeLossResolution, RunStore, TorBroker, new_shared_manager_with_db};

use crate::support::{v2_node_loss, wait_for_path::wait_for_path};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn confirmed_worker_probe_loss_drives_every_durable_node_loss_rule() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("takd.sqlite");
    let socket = temp.path().join("run/takd.sock");
    let store = RunStore::with_db_path(db.clone()).unwrap();
    let runs = v2_node_loss::seed(&store);
    let unavailable = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", unavailable.local_addr().unwrap());
    drop(unavailable);
    let peers = v2_node_loss::peer_manager(&endpoint);
    let manager = new_shared_manager_with_db(db).unwrap();
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let server_socket = crate::support::socket_path::bind_path(&socket);
    let server_store = store.clone();
    let server_peers = peers.clone();
    let remote_inventory = temp.path().join("remotes.toml");
    let server = tokio::spawn(async move {
        takd::run_server_with_local_attempt_executable_and_remote_inventory_until_shutdown(
            &server_socket,
            manager,
            TorBroker::new(),
            server_peers,
            server_store,
            crate::support::takd_bin(),
            remote_inventory,
            shutdown_rx,
        )
        .await
    });
    wait_for_path(&socket, true, "daemon start").await;

    peers.probe_workers_once(&TorBroker::new()).await;
    assert!(peers.scheduler_nodes().is_empty());
    tokio::time::sleep(Duration::from_millis(100)).await;
    for run_id in [&runs.retry, &runs.unsafe_run, &runs.hard, &runs.soft] {
        let run = store.get_run(run_id).unwrap().unwrap();
        assert_eq!(run.jobs[0].state, "running");
        assert_eq!(run.jobs[0].attempt, 1);
        assert_eq!(run.jobs[0].node_id.as_deref(), Some("worker-a"));
    }

    peers.probe_workers_once(&TorBroker::new()).await;
    peers.mark_worker_snapshot("worker-b", v2_node_loss::snapshot("worker-b"));
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let retry = store.get_run(&runs.retry).unwrap().unwrap();
            let unsafe_run = store.get_run(&runs.unsafe_run).unwrap().unwrap();
            let hard = store.get_run(&runs.hard).unwrap().unwrap();
            let soft = store.get_run(&runs.soft).unwrap().unwrap();
            if retry.jobs[0].attempt == 2
                && retry.jobs[0].node_id.as_deref() == Some("worker-b")
                && unsafe_run.jobs[0].state == "failed"
                && unsafe_run.jobs[0].attempt == 1
                && unsafe_run.jobs[0].node_id.as_deref() == Some("worker-a")
                && hard.jobs[0].state == "failed"
                && hard.jobs[0].attempt == 1
                && hard.jobs[0].node_id.as_deref() == Some("worker-a")
                && soft.jobs[0].attempt == 2
                && soft.jobs[0].node_id.as_deref() == Some("worker-b")
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("peer loss did not reach the durable scheduler");
    assert_node_loss_is_durable(&store).await;

    shutdown_tx.send(()).unwrap();
    server.await.unwrap().unwrap();
}

async fn assert_node_loss_is_durable(store: &RunStore) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match store.declare_node_lost("worker-a") {
                Ok(NodeLossResolution::Duplicate) => break,
                Ok(NodeLossResolution::Applied) => panic!("second probe did not declare loss"),
                Err(_) => tokio::time::sleep(Duration::from_millis(10)).await,
            }
        }
    })
    .await
    .expect("durable node loss stayed locked");
}
