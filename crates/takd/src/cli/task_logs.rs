use std::io::{self, Write};
use std::path::Path;
use std::time::Duration;

use anyhow::{Result, anyhow};
use base64::Engine;
use takd::SubmitAttemptStore;

pub(super) fn print_task_logs(
    state_root: &Path,
    task_run_id: &str,
    follow: bool,
    interval_ms: u64,
) -> Result<()> {
    let store = SubmitAttemptStore::with_db_path(state_root.join("agent.sqlite"))?;
    let key = store
        .latest_submit_idempotency_key_for_task_run(task_run_id)?
        .ok_or_else(|| anyhow!("task_run_id {task_run_id} not found in takd task store"))?;
    TaskLogFollower::new(store, key, follow, interval_ms.max(1)).run()
}

struct TaskLogFollower {
    store: SubmitAttemptStore,
    idempotency_key: String,
    follow: bool,
    interval: Duration,
    last_seq: u64,
    polls: usize,
    max_polls: Option<usize>,
}

impl TaskLogFollower {
    fn new(
        store: SubmitAttemptStore,
        idempotency_key: String,
        follow: bool,
        interval_ms: u64,
    ) -> Self {
        Self {
            store,
            idempotency_key,
            follow,
            interval: Duration::from_millis(interval_ms),
            last_seq: 0,
            polls: 0,
            max_polls: test_max_polls(),
        }
    }

    fn run(&mut self) -> Result<()> {
        loop {
            let terminal_event_seen = self.print_new_events()?;
            if !self.follow || terminal_event_seen || self.result_exists()? {
                return Ok(());
            }
            self.polls = self.polls.saturating_add(1);
            if self.max_polls.is_some_and(|limit| self.polls >= limit) {
                return Ok(());
            }
            std::thread::sleep(self.interval);
        }
    }

    fn print_new_events(&mut self) -> Result<bool> {
        let mut terminal_event_seen = false;
        for event in self.store.events(&self.idempotency_key)? {
            if event.seq <= self.last_seq {
                continue;
            }
            self.last_seq = event.seq;
            terminal_event_seen |= print_log_event(&event.payload_json)?;
        }
        Ok(terminal_event_seen)
    }

    fn result_exists(&self) -> Result<bool> {
        Ok(self.store.result_payload(&self.idempotency_key)?.is_some())
    }
}

fn print_log_event(payload_json: &str) -> Result<bool> {
    let payload = serde_json::from_str::<serde_json::Value>(payload_json)
        .unwrap_or_else(|_| serde_json::json!({}));
    let kind = payload
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    match kind {
        "TASK_STDOUT_CHUNK" => write_chunk(&mut io::stdout(), &payload)?,
        "TASK_STDERR_CHUNK" => write_chunk(&mut io::stderr(), &payload)?,
        _ => {}
    }
    Ok(matches!(
        kind,
        "TASK_COMPLETED" | "TASK_FAILED" | "TASK_TERMINAL"
    ))
}

fn write_chunk(writer: &mut impl Write, payload: &serde_json::Value) -> Result<()> {
    writer.write_all(&chunk_bytes(payload)?)?;
    writer.flush()?;
    Ok(())
}

fn chunk_bytes(payload: &serde_json::Value) -> Result<Vec<u8>> {
    if let Some(encoded) = payload
        .get("chunk_base64")
        .and_then(serde_json::Value::as_str)
    {
        return base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(Into::into);
    }
    Ok(payload
        .get("chunk")
        .and_then(serde_json::Value::as_str)
        .map(|value| value.as_bytes().to_vec())
        .unwrap_or_default())
}

fn test_max_polls() -> Option<usize> {
    std::env::var("TAKD_TEST_TASK_LOGS_MAX_POLLS")
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
}
