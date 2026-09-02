use anyhow::{Result, bail};
use futures::future::BoxFuture;

use super::attempt_coordinator::{AttemptDispatch, AttemptObservation, AttemptTransport};
use super::peer_manager::PeerManager;
use super::protocol::TorBroker;
use super::run_store::RunStore;
use super::scheduler::DispatchCommand;
use super::worker_registry::WorkerConnectionTarget;

mod dispatch;
mod observation;
mod outputs;
mod request;
mod workspace_cache;
mod workspace_transfers;

pub struct RemoteAttemptTransport {
    store: RunStore,
    broker: TorBroker,
    peers: PeerManager,
    workspace_transfers: workspace_transfers::WorkspaceTransfers,
}

impl RemoteAttemptTransport {
    #[must_use]
    pub fn new(store: RunStore, broker: TorBroker, peers: PeerManager) -> Self {
        Self {
            store,
            broker,
            peers,
            workspace_transfers: workspace_transfers::WorkspaceTransfers::default(),
        }
    }

    fn target(&self, command: &DispatchCommand) -> Result<WorkerConnectionTarget> {
        let transport = command
            .transport
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("remote attempt has no persisted transport"))?;
        self.peers
            .worker_target(&command.node_id, transport)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "configured worker `{}` no longer has transport `{transport}`",
                    command.node_id
                )
            })
    }
}

impl AttemptTransport for RemoteAttemptTransport {
    fn dispatch<'a>(
        &'a self,
        command: &'a DispatchCommand,
    ) -> BoxFuture<'a, Result<AttemptDispatch>> {
        Box::pin(dispatch::send(self, command))
    }

    fn cancel_and_wait<'a>(&'a self, command: &'a DispatchCommand) -> BoxFuture<'a, Result<()>> {
        Box::pin(observation::cancel(self, command))
    }

    fn reconcile<'a>(
        &'a self,
        command: &'a DispatchCommand,
    ) -> BoxFuture<'a, Result<AttemptObservation>> {
        Box::pin(observation::reconcile(self, command))
    }

    fn acknowledge_terminal<'a>(
        &'a self,
        command: &'a DispatchCommand,
        terminal_digest: &'a str,
        run_terminal: bool,
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(observation::acknowledge(
            self,
            command,
            terminal_digest,
            run_terminal,
        ))
    }
}

fn require_status(status: u16, allowed: &[u16], operation: &str) -> Result<()> {
    if allowed.contains(&status) {
        return Ok(());
    }
    bail!("worker {operation} returned HTTP {status}")
}
