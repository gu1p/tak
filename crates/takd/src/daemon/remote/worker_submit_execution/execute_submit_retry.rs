fn can_retry(member: &RemoteWorkerFusedMember, attempt: u32, exit_code: Option<i32>) -> bool {
    attempt < member.retry.attempts.max(1) && should_retry(exit_code, &member.retry.on_exit)
}

fn should_retry(exit_code: Option<i32>, retry_on_exit: &[i32]) -> bool {
    if retry_on_exit.is_empty() {
        return true;
    }
    exit_code.is_some_and(|code| retry_on_exit.contains(&code))
}

async fn wait_before_retry(member: &RemoteWorkerFusedMember, attempt: u32) {
    let wait = retry_backoff_delay(&member.retry.backoff, attempt);
    if !wait.is_zero() {
        tokio::time::sleep(wait).await;
    }
}

fn retry_backoff_delay(
    backoff: &tak_core::model::BackoffDef,
    attempt: u32,
) -> std::time::Duration {
    match backoff {
        tak_core::model::BackoffDef::Fixed { seconds } => seconds_to_duration(*seconds),
        tak_core::model::BackoffDef::ExpJitter { min_s, max_s, .. } => {
            let exponent = attempt.saturating_sub(1).min(20);
            let factor = 1u64 << exponent;
            let delay = (min_s * factor as f64).min(*max_s);
            seconds_to_duration(delay)
        }
    }
}

fn seconds_to_duration(seconds: f64) -> std::time::Duration {
    if !seconds.is_finite() || seconds <= 0.0 {
        return std::time::Duration::ZERO;
    }
    std::time::Duration::from_secs_f64(seconds)
}
