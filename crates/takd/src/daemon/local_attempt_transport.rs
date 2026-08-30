use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::{Result, bail};
use futures::future::BoxFuture;
use tak_runner::RunCancellation;
use tokio::sync::Notify;

use super::attempt_coordinator::{AttemptObservation, AttemptTransport};
use super::run_store::RunStore;
use super::scheduler::{AttemptCompletion, DispatchCommand};

mod execute;
mod workspace;

#[derive(Clone, PartialEq, Eq, Hash)]
struct AttemptKey {
    run_id: String,
    job_id: String,
    authored_attempt: u32,
    dispatch_generation: u32,
    fencing_token: String,
}

impl From<&DispatchCommand> for AttemptKey {
    fn from(command: &DispatchCommand) -> Self {
        Self {
            run_id: command.run_id.clone(),
            job_id: command.job_id.clone(),
            authored_attempt: command.authored_attempt,
            dispatch_generation: command.dispatch_generation,
            fencing_token: command.fencing_token.clone(),
        }
    }
}

pub(super) struct ActiveAttempt {
    pub(super) cancellation: RunCancellation,
    pub(super) completion: Mutex<Option<AttemptCompletion>>,
    pub(super) completed: Notify,
}

impl ActiveAttempt {
    fn running() -> Arc<Self> {
        Arc::new(Self {
            cancellation: RunCancellation::new(),
            completion: Mutex::new(None),
            completed: Notify::new(),
        })
    }

    fn observation(&self) -> Result<AttemptObservation> {
        Ok(match self.completion.lock().map_err(lock_error)?.clone() {
            Some(completion) => AttemptObservation::Completed(completion),
            None => AttemptObservation::Running,
        })
    }
}

pub(crate) struct LocalAttemptTransport {
    store: RunStore,
    attempts: Mutex<HashMap<AttemptKey, Arc<ActiveAttempt>>>,
}

impl LocalAttemptTransport {
    pub(crate) fn new(store: RunStore) -> Self {
        Self {
            store,
            attempts: Mutex::new(HashMap::new()),
        }
    }

    fn active(&self, command: &DispatchCommand) -> Result<Option<Arc<ActiveAttempt>>> {
        Ok(self
            .attempts
            .lock()
            .map_err(lock_error)?
            .get(&AttemptKey::from(command))
            .cloned())
    }

    fn remove_active(
        &self,
        command: &DispatchCommand,
        expected: &Arc<ActiveAttempt>,
    ) -> Result<()> {
        let key = AttemptKey::from(command);
        let mut attempts = self.attempts.lock().map_err(lock_error)?;
        if attempts
            .get(&key)
            .is_some_and(|active| Arc::ptr_eq(active, expected))
        {
            attempts.remove(&key);
        }
        Ok(())
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
            if self.active(command)?.is_some() {
                return Ok(());
            }
            let snapshot = self.store.local_execution_snapshot(command)?;
            let prepared =
                tokio::task::spawn_blocking(move || workspace::prepare(snapshot)).await??;
            let workspace::Preparation::Execute {
                snapshot,
                workspace_root,
            } = prepared
            else {
                return Ok(());
            };
            if !self.store.local_attempt_is_current(command)? {
                return Ok(());
            }
            workspace::mark_started(&snapshot.attempt_root)?;
            let active = ActiveAttempt::running();
            self.attempts
                .lock()
                .map_err(lock_error)?
                .insert(AttemptKey::from(command), Arc::clone(&active));
            execute::spawn(
                self.store.clone(),
                command.clone(),
                snapshot,
                workspace_root,
                active,
            );
            Ok(())
        })
    }

    fn cancel_and_wait<'a>(&'a self, command: &'a DispatchCommand) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            let Some(active) = self.active(command)? else {
                return Ok(());
            };
            active.cancellation.cancel();
            loop {
                let notified = active.completed.notified();
                if active.completion.lock().map_err(lock_error)?.is_some() {
                    self.remove_active(command, &active)?;
                    return Ok(());
                }
                notified.await;
            }
        })
    }

    fn reconcile<'a>(
        &'a self,
        command: &'a DispatchCommand,
    ) -> BoxFuture<'a, Result<AttemptObservation>> {
        Box::pin(async move {
            if let Some(active) = self.active(command)? {
                let observation = active.observation()?;
                if matches!(observation, AttemptObservation::Completed(_))
                    && workspace::read_completion(&self.store.attempt_root(command))?.is_some()
                {
                    self.remove_active(command, &active)?;
                }
                return Ok(observation);
            }
            Ok(
                workspace::read_completion(&self.store.attempt_root(command))?
                    .map_or(AttemptObservation::Missing, AttemptObservation::Completed),
            )
        })
    }
}

fn lock_error<T>(_error: std::sync::PoisonError<T>) -> anyhow::Error {
    anyhow::anyhow!("local attempt registry lock poisoned")
}
