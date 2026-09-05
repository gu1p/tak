use std::sync::Arc;

use anyhow::Result;

use crate::daemon::attempt_coordinator::{
    AttemptDispatch, AttemptDriveReport, AttemptObservation, AttemptTransport,
};
use crate::daemon::run_store::{RunStore, remote_attempts::WorkerTerminalAck};
use crate::daemon::scheduler::{DispatchCommand, ResultAcceptance};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Key {
    pub(super) run_id: String,
    pub(super) job_id: String,
    pub(super) node_id: String,
    authored_attempt: u32,
    dispatch_generation: u32,
    fencing_token: String,
    pub(super) operation: &'static str,
    acknowledgement: Option<(String, bool)>,
}

pub(super) enum Operation {
    Dispatch(DispatchCommand),
    Observe(DispatchCommand),
    Acknowledge(WorkerTerminalAck),
}

impl Operation {
    pub(super) fn key(&self) -> Key {
        let (command, operation, acknowledgement) = match self {
            Self::Dispatch(command) => (command, "dispatch", None),
            Self::Observe(command) => (command, "observation", None),
            Self::Acknowledge(ack) => (
                &ack.command,
                "terminal acknowledgement",
                Some((ack.terminal_digest.clone(), ack.run_terminal)),
            ),
        };
        Key {
            run_id: command.run_id.clone(),
            job_id: command.job_id.clone(),
            node_id: command.node_id.clone(),
            authored_attempt: command.authored_attempt,
            dispatch_generation: command.dispatch_generation,
            fencing_token: command.fencing_token.clone(),
            operation,
            acknowledgement,
        }
    }

    pub(super) fn prepare(&self, store: &RunStore) -> Result<bool> {
        match self {
            Self::Dispatch(command) => {
                Ok(store.mark_dispatch_started(command)? != ResultAcceptance::Stale)
            }
            _ => Ok(true),
        }
    }

    pub(super) async fn execute<T: AttemptTransport>(
        self,
        transport: Arc<T>,
    ) -> Result<Completion> {
        match self {
            Self::Dispatch(command) => {
                let result = transport.dispatch(&command).await?;
                Ok(Completion::Dispatched(command, result))
            }
            Self::Observe(command) => {
                let result = transport.reconcile(&command).await?;
                Ok(Completion::Observed(command, result))
            }
            Self::Acknowledge(ack) => {
                transport
                    .acknowledge_terminal(&ack.command, &ack.terminal_digest, ack.run_terminal)
                    .await?;
                Ok(Completion::Acknowledged(ack))
            }
        }
    }
}

pub(super) enum Completion {
    Dispatched(DispatchCommand, AttemptDispatch),
    Observed(DispatchCommand, AttemptObservation),
    Acknowledged(WorkerTerminalAck),
}

impl Completion {
    pub(super) fn apply(self, store: &RunStore, report: &mut AttemptDriveReport) -> Result<()> {
        match self {
            Self::Dispatched(command, AttemptDispatch::Accepted) => {
                store.ack_dispatch(&command)?;
                report.dispatched += 1;
            }
            Self::Dispatched(command, AttemptDispatch::Stale)
            | Self::Observed(command, AttemptObservation::Missing) => {
                store.resolve_unknown_attempt(&command)?;
                report.reconciled += 1;
            }
            Self::Observed(command, AttemptObservation::Completed(completion)) => {
                store.complete_attempt(&command, completion)?;
                report.reconciled += 1;
            }
            Self::Observed(_, AttemptObservation::Running) => report.reconciled += 1,
            Self::Acknowledged(ack) => {
                store.mark_worker_terminal_acknowledged(&ack)?;
                report.acknowledged += 1;
            }
        }
        Ok(())
    }
}
