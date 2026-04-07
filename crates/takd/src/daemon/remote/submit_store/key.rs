use super::*;

/// Builds a deterministic submit idempotency key from `task_run_id` and `attempt`.
///
/// ```compile_fail
/// // Reason: rustdoc links full `takd` runnable doctests, and this crate currently trips a nightly `rust-lld` bus error during link.
/// # use takd::daemon::remote::build_submit_idempotency_key;
/// # let _force_compile_fail: () = 1;
/// let key = build_submit_idempotency_key("run-123", Some(2)).unwrap();
/// assert_eq!(key, "run-123:2");
/// ```
pub fn build_submit_idempotency_key(task_run_id: &str, attempt: Option<u32>) -> Result<String> {
    let task_run_id = task_run_id.trim();
    if task_run_id.is_empty() {
        bail!("task_run_id is required");
    }

    let attempt = validate_submit_attempt(attempt)?;
    Ok(format!("{task_run_id}:{attempt}"))
}

pub(super) fn validate_submit_attempt(attempt: Option<u32>) -> Result<u32> {
    let attempt = attempt.ok_or_else(|| anyhow!("submit idempotency attempt is required"))?;
    if attempt == 0 {
        bail!("submit idempotency attempt must be >= 1");
    }
    Ok(attempt)
}
