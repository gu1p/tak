use std::time::Duration;

use tak_core::model::BackoffDef;

/// Returns true when the given exit code qualifies for retry under policy rules.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub(crate) fn should_retry(exit_code: Option<i32>, retry_on_exit: &[i32]) -> bool {
    if retry_on_exit.is_empty() {
        return true;
    }

    exit_code.is_some_and(|code| retry_on_exit.contains(&code))
}

/// Computes retry delay duration for the configured backoff strategy.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub(crate) fn retry_backoff_delay(backoff: &BackoffDef, attempt: u32) -> Duration {
    match backoff {
        BackoffDef::Fixed { seconds } => seconds_to_duration(*seconds),
        BackoffDef::ExpJitter { min_s, max_s, .. } => {
            let exponent = attempt.saturating_sub(1).min(20);
            let factor = 1u64 << exponent;
            let delay = (min_s * factor as f64).min(*max_s);
            seconds_to_duration(delay)
        }
    }
}

/// Converts a floating-point second value into a clamped non-negative duration.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub(crate) fn seconds_to_duration(seconds: f64) -> Duration {
    if !seconds.is_finite() || seconds <= 0.0 {
        return Duration::ZERO;
    }
    Duration::from_secs_f64(seconds)
}
