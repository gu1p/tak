use std::sync::Arc;

use anyhow::Result;
use futures::future::BoxFuture;

use super::run_store::RunStore;
use super::scheduler::{AttemptCompletion, DispatchCommand};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttemptObservation {
    Running,
    Completed(AttemptCompletion),
    Missing,
}

pub trait AttemptTransport: Send + Sync {
    fn dispatch<'a>(&'a self, command: &'a DispatchCommand) -> BoxFuture<'a, Result<()>>;
    fn cancel_and_wait<'a>(&'a self, command: &'a DispatchCommand) -> BoxFuture<'a, Result<()>>;
    fn reconcile<'a>(
        &'a self,
        command: &'a DispatchCommand,
    ) -> BoxFuture<'a, Result<AttemptObservation>>;
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AttemptDriveReport {
    pub cancelled: usize,
    pub dispatched: usize,
    pub reconciled: usize,
    pub deferred: usize,
}

pub struct AttemptCoordinator<T> {
    store: RunStore,
    transport: Arc<T>,
}

impl<T: AttemptTransport> AttemptCoordinator<T> {
    #[must_use]
    pub fn new(store: RunStore, transport: Arc<T>) -> Self {
        Self { store, transport }
    }

    pub async fn drive_once(&mut self) -> Result<AttemptDriveReport> {
        let mut report = AttemptDriveReport::default();
        for command in self.store.pending_cancellations()? {
            match self.transport.cancel_and_wait(&command).await {
                Ok(()) => {
                    self.store.ack_cancellation(&command)?;
                    report.cancelled += 1;
                }
                Err(error) => {
                    tracing::debug!("attempt cancellation deferred: {error:#}");
                    report.deferred += 1;
                }
            }
        }
        let running = self.store.running_attempts_for_reconciliation()?;
        for command in self.store.pending_dispatches()? {
            match self.transport.dispatch(&command).await {
                Ok(()) => {
                    self.store.ack_dispatch(&command)?;
                    report.dispatched += 1;
                }
                Err(error) => {
                    tracing::debug!("attempt dispatch deferred: {error:#}");
                    report.deferred += 1;
                }
            }
        }
        for command in running {
            match self.transport.reconcile(&command).await {
                Ok(AttemptObservation::Running) => report.reconciled += 1,
                Ok(AttemptObservation::Completed(completion)) => {
                    self.store.complete_attempt(&command, completion)?;
                    report.reconciled += 1;
                }
                Ok(AttemptObservation::Missing) => {
                    self.store.resolve_unknown_attempt(&command)?;
                    report.reconciled += 1;
                }
                Err(error) => {
                    tracing::debug!("attempt reconciliation deferred: {error:#}");
                    report.deferred += 1;
                }
            }
        }
        Ok(report)
    }
}
