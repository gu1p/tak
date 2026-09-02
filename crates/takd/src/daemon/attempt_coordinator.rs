use std::sync::Arc;

use anyhow::Result;
use futures::future::BoxFuture;

use super::run_store::RunStore;
use super::run_store::remote_attempts::WorkerTerminalAck;
use super::scheduler::{AttemptCompletion, DispatchCommand, ResultAcceptance};

mod cancellations;

use cancellations::InFlightCancellations;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttemptObservation {
    Running,
    Completed(AttemptCompletion),
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttemptDispatch {
    Accepted,
    Stale,
}

pub trait AttemptTransport: Send + Sync {
    fn dispatch<'a>(
        &'a self,
        command: &'a DispatchCommand,
    ) -> BoxFuture<'a, Result<AttemptDispatch>>;
    fn cancel_and_wait<'a>(&'a self, command: &'a DispatchCommand) -> BoxFuture<'a, Result<()>>;
    fn reconcile<'a>(
        &'a self,
        command: &'a DispatchCommand,
    ) -> BoxFuture<'a, Result<AttemptObservation>>;
    fn acknowledge_terminal<'a>(
        &'a self,
        _command: &'a DispatchCommand,
        _terminal_digest: &'a str,
        _run_terminal: bool,
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(async { Ok(()) })
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AttemptDriveReport {
    pub cancelled: usize,
    pub dispatched: usize,
    pub reconciled: usize,
    pub deferred: usize,
    pub acknowledged: usize,
}

pub struct AttemptCoordinator<T> {
    store: RunStore,
    transport: Arc<T>,
    cancellations: InFlightCancellations,
}

impl<T: AttemptTransport + 'static> AttemptCoordinator<T> {
    #[must_use]
    pub fn new(store: RunStore, transport: Arc<T>) -> Self {
        Self {
            store,
            transport,
            cancellations: InFlightCancellations::default(),
        }
    }

    pub async fn drive_once(&mut self) -> Result<AttemptDriveReport> {
        let mut report = AttemptDriveReport::default();
        self.cancellations.collect(&self.store, &mut report)?;
        self.cancellations
            .start(self.transport.clone(), self.store.pending_cancellations()?);
        tokio::task::yield_now().await;
        self.cancellations.collect(&self.store, &mut report)?;
        report.deferred += self.cancellations.len();
        let acknowledgements: Vec<WorkerTerminalAck> = self.store.pending_worker_terminal_acks()?;
        for ack in acknowledgements {
            match self
                .transport
                .acknowledge_terminal(&ack.command, &ack.terminal_digest, ack.run_terminal)
                .await
            {
                Ok(()) => {
                    self.store.mark_worker_terminal_acknowledged(&ack)?;
                    report.acknowledged += 1;
                }
                Err(error) => {
                    tracing::debug!("attempt terminal acknowledgement deferred: {error:#}");
                    report.deferred += 1;
                }
            }
        }
        let running = self.store.running_attempts_for_reconciliation()?;
        for command in self.store.pending_dispatches()? {
            if self.store.mark_dispatch_started(&command)? == ResultAcceptance::Stale {
                continue;
            }
            match self.transport.dispatch(&command).await {
                Ok(AttemptDispatch::Accepted) => {
                    self.store.ack_dispatch(&command)?;
                    report.dispatched += 1;
                }
                Ok(AttemptDispatch::Stale) => {
                    self.store.resolve_unknown_attempt(&command)?;
                    report.reconciled += 1;
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
