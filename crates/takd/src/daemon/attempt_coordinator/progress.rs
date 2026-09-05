use std::collections::BTreeSet;
use std::sync::Arc;

use anyhow::Result;
use futures::future::{BoxFuture, FutureExt};
use futures::stream::{FuturesUnordered, StreamExt};

use super::{AttemptDriveReport, AttemptTransport, RunStore};

mod operation;

use operation::{Completion, Key, Operation};

type PendingOperation = BoxFuture<'static, (Key, Result<Completion>)>;

#[derive(Default)]
pub(super) struct InFlightProgress {
    pending: FuturesUnordered<PendingOperation>,
    keys: BTreeSet<Key>,
}

impl InFlightProgress {
    pub(super) fn start<T: AttemptTransport + 'static>(
        &mut self,
        transport: Arc<T>,
        store: &RunStore,
    ) -> Result<()> {
        let running = store.running_attempts_for_reconciliation()?;
        let operations = store
            .pending_worker_terminal_acks()?
            .into_iter()
            .map(Operation::Acknowledge)
            .chain(
                store
                    .pending_dispatches()?
                    .into_iter()
                    .map(Operation::Dispatch),
            )
            .chain(running.into_iter().map(Operation::Observe));
        for operation in operations {
            let key = operation.key();
            if self.keys.contains(&key) || !operation.prepare(store)? {
                continue;
            }
            self.keys.insert(key.clone());
            let transport = transport.clone();
            self.pending
                .push(async move { (key, operation.execute(transport).await) }.boxed());
        }
        Ok(())
    }

    pub(super) fn collect(
        &mut self,
        store: &RunStore,
        report: &mut AttemptDriveReport,
    ) -> Result<()> {
        while let Some(Some((key, result))) = self.pending.next().now_or_never() {
            self.keys.remove(&key);
            match result {
                Ok(completion) => completion.apply(store, report)?,
                Err(error) => {
                    tracing::debug!(
                        run_id = %key.run_id, job_id = %key.job_id, node_id = %key.node_id,
                        operation = key.operation, error = %format!("{error:#}"),
                        "attempt operation deferred"
                    );
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
