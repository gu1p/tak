use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::task::JoinHandle;

use super::attempt_coordinator::AttemptCoordinator;
use super::run_store::RunStore;
use super::scheduler::SchedulerNode;
use super::{DaemonAttemptTransport, LocalAttemptTransport, RemoteAttemptTransport};

pub(crate) struct RunDriver {
    task: Option<JoinHandle<()>>,
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
}

impl RunDriver {
    pub(crate) fn spawn(
        store: RunStore,
        local_attempt_executable: PathBuf,
        broker: super::protocol::TorBroker,
        peers: super::peer_manager::PeerManager,
    ) -> Self {
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let capacity = std::thread::available_parallelism()
                .map_or(1, |parallelism| parallelism.get())
                .try_into()
                .unwrap_or(u32::MAX);
            let local_node = SchedulerNode::with_execution_slots("local", capacity);
            let transport = Arc::new(DaemonAttemptTransport::new(
                LocalAttemptTransport::new(store.clone(), local_attempt_executable),
                RemoteAttemptTransport::new(store.clone(), broker, peers.clone()),
            ));
            let mut coordinator = AttemptCoordinator::new(store.clone(), transport);
            loop {
                if shutdown_requested(&mut shutdown_rx) {
                    break;
                }
                drive_worker_node_losses(&store, &peers);
                let mut nodes = vec![local_node.clone()];
                nodes.extend(peers.scheduler_nodes());
                if let Err(error) = schedule_ready(&store, &nodes) {
                    tracing::error!("v2 scheduler tick failed: {error:#}");
                }
                let driven = tokio::select! {
                    biased;
                    _ = &mut shutdown_rx => break,
                    result = coordinator.drive_once() => result,
                };
                if let Err(error) = driven {
                    tracing::error!("v2 attempt driver tick failed: {error:#}");
                }
                tokio::select! {
                    _ = &mut shutdown_rx => break,
                    _ = tokio::time::sleep(Duration::from_millis(20)) => {}
                }
            }
        });
        Self {
            task: Some(task),
            shutdown: Some(shutdown_tx),
        }
    }

    pub(crate) fn request_shutdown(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
    }

    pub(crate) async fn shutdown(mut self) {
        self.request_shutdown();
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
    }
}

impl Drop for RunDriver {
    fn drop(&mut self) {
        self.request_shutdown();
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

fn shutdown_requested(shutdown: &mut tokio::sync::oneshot::Receiver<()>) -> bool {
    !matches!(
        shutdown.try_recv(),
        Err(tokio::sync::oneshot::error::TryRecvError::Empty)
    )
}

fn schedule_ready(store: &RunStore, nodes: &[SchedulerNode]) -> anyhow::Result<()> {
    if !store.has_ready_jobs()? {
        return Ok(());
    }
    while store.reserve_next(nodes)?.is_some() {}
    Ok(())
}

fn drive_worker_node_losses(store: &RunStore, peers: &super::peer_manager::PeerManager) {
    for node_id in peers.pending_worker_node_losses() {
        match store.declare_node_lost(&node_id) {
            Ok(_) => peers.acknowledge_worker_node_loss(&node_id),
            Err(error) => tracing::error!(node_id, "v2 worker loss handling failed: {error:#}"),
        }
    }
    for node_id in peers.pending_worker_probe_failures() {
        match has_active_attempt_on_node(store, &node_id) {
            Ok(true) => match store.declare_node_lost(&node_id) {
                Ok(_) => peers.acknowledge_worker_probe_failure(&node_id),
                Err(error) => {
                    tracing::error!(node_id, "v2 worker loss handling failed: {error:#}")
                }
            },
            Ok(false) => peers.acknowledge_worker_probe_failure(&node_id),
            Err(error) => tracing::error!(node_id, "v2 worker loss check failed: {error:#}"),
        }
    }
}

fn has_active_attempt_on_node(store: &RunStore, node_id: &str) -> anyhow::Result<bool> {
    let pending = store.pending_dispatches()?;
    let running = store.running_attempts_for_reconciliation()?;
    let cancelling = store.pending_cancellations()?;
    Ok(pending
        .iter()
        .chain(&running)
        .chain(&cancelling)
        .any(|command| command.node_id == node_id))
}
