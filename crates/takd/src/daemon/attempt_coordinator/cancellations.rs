use std::collections::BTreeSet;
use std::sync::Arc;

use anyhow::Result;
use futures::future::{BoxFuture, FutureExt};
use futures::stream::{FuturesUnordered, StreamExt};

use super::{AttemptDriveReport, AttemptTransport, DispatchCommand, RunStore};

type PendingCancellation = BoxFuture<'static, (String, DispatchCommand, Result<()>)>;

#[derive(Default)]
pub(super) struct InFlightCancellations {
    pending: FuturesUnordered<PendingCancellation>,
    keys: BTreeSet<String>,
}

impl InFlightCancellations {
    pub(super) fn start<T: AttemptTransport + 'static>(
        &mut self,
        transport: Arc<T>,
        commands: Vec<DispatchCommand>,
    ) {
        for command in commands {
            let key = key(&command);
            if !self.keys.insert(key.clone()) {
                continue;
            }
            let transport = transport.clone();
            self.pending.push(
                async move {
                    let result = transport.cancel_and_wait(&command).await;
                    (key, command, result)
                }
                .boxed(),
            );
        }
    }

    pub(super) fn collect(
        &mut self,
        store: &RunStore,
        report: &mut AttemptDriveReport,
    ) -> Result<()> {
        loop {
            let Some(Some((key, command, result))) = self.pending.next().now_or_never() else {
                break;
            };
            self.keys.remove(&key);
            match result {
                Ok(()) => {
                    store.ack_cancellation(&command)?;
                    report.cancelled += 1;
                }
                Err(error) => {
                    tracing::debug!("attempt cancellation deferred: {error:#}");
                    report.deferred += 1;
                }
            }
        }
        Ok(())
    }

    pub(super) fn len(&self) -> usize {
        self.pending.len()
    }
}

fn key(command: &DispatchCommand) -> String {
    format!(
        "{}/{}/{}/{}/{}",
        command.run_id,
        command.job_id,
        command.authored_attempt,
        command.dispatch_generation,
        command.fencing_token
    )
}
