use super::*;

const ABANDONED_MESSAGE: &str = "task abandoned because takd restarted before it completed";

impl SubmitAttemptStore {
    /// Marks unfinished submit attempts as abandoned after a daemon restart.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on local sqlite availability and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub fn mark_unfinished_attempts_abandoned(&self) -> Result<usize> {
        let active = self.active_attempts()?;
        for attempt in &active {
            self.mark_attempt_abandoned(attempt)?;
        }
        Ok(active.len())
    }

    fn mark_attempt_abandoned(&self, attempt: &ActiveSubmitAttempt) -> Result<()> {
        let finished_at = unix_epoch_ms();
        self.set_result_payload(
            &attempt.idempotency_key,
            &abandoned_result_payload(attempt, finished_at),
        )?;
        self.append_event(
            &attempt.idempotency_key,
            self.next_event_seq(&attempt.idempotency_key)?,
            &abandoned_event_payload(finished_at),
        )
    }
}

fn abandoned_result_payload(attempt: &ActiveSubmitAttempt, finished_at: i64) -> String {
    serde_json::json!({
        "success": false,
        "exit_code": 1,
        "started_at": attempt.created_at_ms,
        "finished_at": finished_at,
        "duration_ms": finished_at.saturating_sub(attempt.created_at_ms),
        "transport_kind": "direct",
        "sync_mode": "OUTPUTS_AND_LOGS",
        "outputs": [],
        "stderr_tail": ABANDONED_MESSAGE,
    })
    .to_string()
}

fn abandoned_event_payload(finished_at: i64) -> String {
    serde_json::json!({
        "kind": "TASK_FAILED",
        "timestamp_ms": finished_at,
        "success": false,
        "exit_code": 1,
        "message": ABANDONED_MESSAGE,
    })
    .to_string()
}
