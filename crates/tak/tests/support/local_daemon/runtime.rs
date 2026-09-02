use std::sync::mpsc::{self, Receiver};
use std::{path::PathBuf, time::Duration};
use takd::run_server_with_local_attempt_executable_and_remote_inventory_until_shutdown as run_server;
use takd::{PeerManager, RunStore, TorBroker};
pub(super) struct ServerThread {
    pub(super) thread: std::thread::JoinHandle<()>,
    pub(super) stopped: Receiver<Result<(), String>>,
    pub(super) shutdown: tokio::sync::oneshot::Sender<()>,
}
pub(super) fn spawn_server(
    socket_path: PathBuf,
    manager: takd::SharedLeaseManager,
    broker: TorBroker,
    peers: PeerManager,
    db_path: PathBuf,
    attempt_executable: PathBuf,
    remote_inventory_path: PathBuf,
) -> Result<ServerThread, String> {
    let (ready_tx, ready_rx) = mpsc::channel();
    let (stopped_tx, stopped_rx) = mpsc::channel();
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let thread = std::thread::Builder::new()
        .name("tak-test-daemon".into())
        .spawn(move || {
            let runtime = isolated_runtime();
            let completion = match RunStore::with_db_path(db_path) {
                Ok(run_store) => runtime.block_on(run(
                    socket_path,
                    manager,
                    broker,
                    peers,
                    run_store,
                    attempt_executable,
                    remote_inventory_path,
                    shutdown_rx,
                    ready_tx,
                )),
                Err(error) => {
                    let message = format!("open daemon run store: {error:#}");
                    let _ = ready_tx.send(Err(message.clone()));
                    Err(message)
                }
            };
            drop(runtime);
            let _ = stopped_tx.send(completion);
        })
        .map_err(|error| format!("spawn daemon thread: {error}"))?;
    ready_rx
        .recv_timeout(Duration::from_secs(30))
        .map_err(|error| format!("wait for daemon readiness: {error}"))??;
    Ok(ServerThread {
        thread,
        stopped: stopped_rx,
        shutdown: shutdown_tx,
    })
}
#[allow(clippy::too_many_arguments)]
async fn run(
    socket_path: PathBuf,
    manager: takd::SharedLeaseManager,
    broker: TorBroker,
    peers: PeerManager,
    run_store: RunStore,
    attempt_executable: PathBuf,
    remote_inventory_path: PathBuf,
    shutdown: tokio::sync::oneshot::Receiver<()>,
    ready: mpsc::Sender<Result<(), String>>,
) -> Result<(), String> {
    let server = run_server(
        &socket_path,
        manager,
        broker.clone(),
        peers.clone(),
        run_store,
        attempt_executable,
        remote_inventory_path,
        shutdown,
    );
    tokio::pin!(server);
    while !socket_path.exists() {
        tokio::select! {
            result = &mut server => {
                let result = result.map_err(|error| format!("{error:#}"));
                let _ = ready.send(Err(result.as_ref().err().cloned().unwrap_or_else(||
                    "server exited before its socket appeared".into())));
                return result;
            }
            _ = tokio::time::sleep(Duration::from_millis(10)) => {}
        }
    }
    peers.probe_workers_once(&broker).await;
    let _ = ready.send(Ok(()));
    server.await.map_err(|error| format!("{error:#}"))
}
fn isolated_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("isolated local daemon runtime")
}
