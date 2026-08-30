use std::path::PathBuf;

use anyhow::{Result, bail};
use futures::future::BoxFuture;

use super::attempt_coordinator::{AttemptObservation, AttemptTransport};
use super::run_store::RunStore;
use super::scheduler::DispatchCommand;

mod durable_state;
mod execute;
mod launcher;
mod workspace;
mod wrapper;

#[cfg(test)]
mod durable_state_tests;
#[cfg(test)]
mod launcher_tests;
#[cfg(test)]
mod workspace_tests;

pub use wrapper::run_local_attempt_subprocess;

pub(crate) struct LocalAttemptTransport {
    store: RunStore,
    executable: PathBuf,
}

impl LocalAttemptTransport {
    pub(crate) fn new(store: RunStore, executable: PathBuf) -> Self {
        Self { store, executable }
    }
}

impl AttemptTransport for LocalAttemptTransport {
    fn dispatch<'a>(&'a self, command: &'a DispatchCommand) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            if command.node_id != "local" {
                bail!(
                    "local attempt transport cannot dispatch node `{}`",
                    command.node_id
                );
            }
            launcher::dispatch(&self.store, &self.executable, command).await
        })
    }

    fn cancel_and_wait<'a>(&'a self, command: &'a DispatchCommand) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            let root = self.store.attempt_root(command);
            loop {
                match durable_state::observe(&root)? {
                    AttemptObservation::Running => {
                        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                    }
                    AttemptObservation::Completed(_) | AttemptObservation::Missing => return Ok(()),
                }
            }
        })
    }

    fn reconcile<'a>(
        &'a self,
        command: &'a DispatchCommand,
    ) -> BoxFuture<'a, Result<AttemptObservation>> {
        Box::pin(async move { durable_state::observe(&self.store.attempt_root(command)) })
    }
}
