use std::sync::Arc;
use std::time::Duration;

use tokio::task::JoinHandle;

use super::LocalAttemptTransport;
use super::attempt_coordinator::AttemptCoordinator;
use super::run_store::RunStore;
use super::scheduler::SchedulerNode;

pub(crate) struct RunDriver {
    task: JoinHandle<()>,
}

impl RunDriver {
    pub(crate) fn spawn(store: RunStore) -> Self {
        let task = tokio::spawn(async move {
            let capacity = std::thread::available_parallelism()
                .map_or(1, |parallelism| parallelism.get())
                .try_into()
                .unwrap_or(u32::MAX);
            let nodes = [SchedulerNode::with_execution_slots("local", capacity)];
            let transport = Arc::new(LocalAttemptTransport::new(store.clone()));
            let mut coordinator = AttemptCoordinator::new(store.clone(), transport);
            loop {
                if let Err(error) = schedule_ready(&store, &nodes) {
                    tracing::error!("local v2 scheduler tick failed: {error:#}");
                }
                if let Err(error) = coordinator.drive_once().await {
                    tracing::error!("local v2 attempt driver tick failed: {error:#}");
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        });
        Self { task }
    }
}

impl Drop for RunDriver {
    fn drop(&mut self) {
        self.task.abort();
    }
}

fn schedule_ready(store: &RunStore, nodes: &[SchedulerNode]) -> anyhow::Result<()> {
    if !store.has_ready_jobs()? {
        return Ok(());
    }
    while store.reserve_next(nodes)?.is_some() {}
    Ok(())
}
