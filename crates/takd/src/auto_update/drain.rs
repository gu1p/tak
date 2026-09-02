//! Wait for in-flight remote work to finish before swapping the daemon binary.

use std::time::Duration;

use crate::daemon::remote::SubmitAttemptStore;

/// Outcome of waiting for the agent to become idle.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum DrainOutcome {
    /// No active submit attempts remained.
    Idle,
    /// The deadline elapsed while work was still in flight.
    DeadlineExceeded,
}

impl DrainOutcome {
    pub(crate) fn allows_replacement(&self) -> bool {
        matches!(self, Self::Idle)
    }
}

/// Poll the submit store until no active attempts remain, or `deadline` elapses.
///
/// The deadline bounds each drain attempt. Its caller leaves the update pending
/// when work remains and retries during a later update tick.
///
/// ```no_run
/// # // Reason: needs a tokio runtime and a constructed SubmitAttemptStore backed by sqlite.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub(crate) async fn wait_until_idle(
    store: &SubmitAttemptStore,
    deadline: Duration,
) -> DrainOutcome {
    let start = tokio::time::Instant::now();
    loop {
        let active = match store.active_attempts() {
            Ok(attempts) => attempts.len(),
            Err(err) => {
                // A store error must NOT be read as "idle" — work may still be
                // running. Treat as busy and let the deadline decide.
                tracing::warn!(
                    "auto-update drain: active_attempts failed; treating as busy: {err:#}"
                );
                usize::MAX
            }
        };
        if active == 0 {
            return DrainOutcome::Idle;
        }
        if start.elapsed() >= deadline {
            return DrainOutcome::DeadlineExceeded;
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}
