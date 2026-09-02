use std::future::Future;
use std::pin::Pin;

use tokio::task::JoinSet;

use super::*;
use crate::daemon::RunDriver;
use crate::daemon::protocol::server_background_task::ServerBackgroundTask;

pub(super) type ServerShutdown = Pin<Box<dyn Future<Output = ()> + Send>>;

#[allow(clippy::too_many_arguments)]
pub(super) async fn serve(
    listener: UnixListener,
    manager: SharedLeaseManager,
    broker: TorBroker,
    peers: crate::daemon::peer_manager::PeerManager,
    run_store: RunStore,
    local_attempt_executable: PathBuf,
    remote_access: crate::daemon::RemoteAccess,
    shutdown: ServerShutdown,
) -> Result<()> {
    let mut driver = RunDriver::spawn(
        run_store.clone(),
        local_attempt_executable,
        broker.clone(),
        peers.clone(),
    );
    let maintenance = ServerBackgroundTask::spawn(run_store.clone().maintain_periodically());
    let warmup = spawn_broker_warmup(broker.clone());
    let mut clients = JoinSet::new();
    let result = accept_until_shutdown(
        &listener,
        &manager,
        &peers,
        &run_store,
        &remote_access,
        &mut clients,
        shutdown,
    )
    .await;
    driver.request_shutdown();
    maintenance.abort();
    warmup.abort();
    driver.shutdown().await;
    clients.shutdown().await;
    maintenance.shutdown().await;
    warmup.shutdown().await;
    result
}

#[allow(clippy::too_many_arguments)]
async fn accept_until_shutdown(
    listener: &UnixListener,
    manager: &SharedLeaseManager,
    peers: &crate::daemon::peer_manager::PeerManager,
    run_store: &RunStore,
    remote_access: &crate::daemon::RemoteAccess,
    clients: &mut JoinSet<()>,
    mut shutdown: ServerShutdown,
) -> Result<()> {
    loop {
        tokio::select! {
            _ = &mut shutdown => return Ok(()),
            joined = clients.join_next(), if !clients.is_empty() => {
                if let Some(Err(error)) = joined {
                    tracing::error!("client task failed: {error}");
                }
            }
            accepted = listener.accept() => {
                let (stream, _) = accepted.context("accept failed")?;
                let manager = Arc::clone(manager);
                let peers = peers.clone();
                let run_store = run_store.clone();
                let remote_access = remote_access.clone();
                clients.spawn(async move {
                    if let Err(error) = handle_client(stream, manager, peers, run_store, remote_access).await {
                        tracing::error!("client handling error: {error}");
                    }
                });
            }
        }
    }
}

fn spawn_broker_warmup(broker: TorBroker) -> ServerBackgroundTask {
    ServerBackgroundTask::spawn(async move {
        if let Err(error) = broker.warm().await {
            tracing::debug!("local Tor broker warmup failed: {error:#}");
        }
    })
}
