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

/// Poll the submit store until no active attempts remain, or `deadline` elapses.
///
/// A stuck task must not pin an old (possibly vulnerable) binary forever, so the
/// caller proceeds anyway after the deadline; the restarted daemon re-adopts
/// orphaned attempts from the same sqlite store.
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
