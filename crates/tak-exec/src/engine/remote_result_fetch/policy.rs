//! Tunable bounds and backoff schedule for the result-fetch retry loop.

use std::time::Duration;

use tak_core::model::BackoffDef;

/// How many times a transient (5xx / retryable transport) result fetch is retried
/// before the failure is surfaced. Deliberately tighter than the event stream's
/// 30 reconnects — a terminal GET should not stay patient for a hard 500.
const RESULT_FETCH_MAX_ATTEMPTS: u32 = 5;
/// How many times a post-`done` 404 is tolerated before declaring the result
/// genuinely missing. Covers the (non-atomic) window where the terminal event is
/// appended but the result row write raced or failed.
const RESULT_NOT_FOUND_GRACE_ATTEMPTS: u32 = 5;
/// Short fixed delay between 404 grace attempts (~1.25s total over the budget).
const RESULT_NOT_FOUND_BACKOFF: Duration = Duration::from_millis(250);

/// Exponential backoff for transient result fetches: 0.25s, 0.5s, 1s, 2s, 4s.
///
/// ```no_run
/// # // Reason: This private policy helper is exercised through result-fetch tests.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn result_fetch_backoff() -> BackoffDef {
    BackoffDef::ExpJitter {
        min_s: 0.25,
        max_s: 4.0,
        jitter: String::from("none"),
    }
}

/// Tunable bounds for the result-fetch retry loop. Production uses
/// [`ResultFetchPolicy::production`]; tests inject a zero-backoff policy.
pub(crate) struct ResultFetchPolicy {
    /// Max transient (5xx / retryable transport) retries before failing.
    pub(crate) max_attempts: u32,
    /// Max post-`done` 404 retries before declaring the result missing.
    pub(crate) not_found_grace: u32,
    /// Backoff for transient retries.
    pub(crate) backoff: BackoffDef,
    /// Fixed delay between 404 grace retries.
    pub(crate) not_found_backoff: Duration,
}

impl ResultFetchPolicy {
    pub(crate) fn production() -> Self {
        Self {
            max_attempts: RESULT_FETCH_MAX_ATTEMPTS,
            not_found_grace: RESULT_NOT_FOUND_GRACE_ATTEMPTS,
            backoff: result_fetch_backoff(),
            not_found_backoff: RESULT_NOT_FOUND_BACKOFF,
        }
    }
}
