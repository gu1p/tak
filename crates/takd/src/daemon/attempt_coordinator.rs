use std::sync::Arc;

use anyhow::Result;
use futures::future::BoxFuture;

use super::run_store::RunStore;
use super::scheduler::{AttemptCompletion, DispatchCommand};

mod cancellations;
mod progress;

use cancellations::InFlightCancellations;
use progress::InFlightProgress;

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
    progress: InFlightProgress,
}

impl<T: AttemptTransport + 'static> AttemptCoordinator<T> {
    #[must_use]
    pub fn new(store: RunStore, transport: Arc<T>) -> Self {
        Self {
            store,
            transport,
            cancellations: InFlightCancellations::default(),
            progress: InFlightProgress::default(),
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
        self.progress.start(self.transport.clone(), &self.store)?;
        self.progress.collect(&self.store, &mut report)?;
        tokio::task::yield_now().await;
        self.progress.collect(&self.store, &mut report)?;
        report.deferred += self.progress.len();
        Ok(report)
    }
}
