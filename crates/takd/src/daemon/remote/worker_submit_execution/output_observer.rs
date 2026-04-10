use base64::Engine;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

const REMOTE_RESULT_TAIL_LIMIT_BYTES: usize = 4096;

#[derive(Clone)]
struct RemoteWorkerEventObserver {
    store: SubmitAttemptStore,
    idempotency_key: String,
    next_seq: Arc<AtomicU64>,
    stdout_tail: Arc<Mutex<Vec<u8>>>,
    stderr_tail: Arc<Mutex<Vec<u8>>>,
}

impl RemoteWorkerEventObserver {
    fn new(store: SubmitAttemptStore, idempotency_key: String) -> Self {
        Self {
            store,
            idempotency_key,
            next_seq: Arc::new(AtomicU64::new(2)),
            stdout_tail: Arc::new(Mutex::new(Vec::new())),
            stderr_tail: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn claim_next_seq(&self) -> u64 {
        self.next_seq.fetch_add(1, Ordering::SeqCst)
    }

    fn stdout_tail(&self) -> String {
        read_tail_buffer(&self.stdout_tail)
    }

    fn stderr_tail(&self) -> String {
        read_tail_buffer(&self.stderr_tail)
    }
}

impl TaskOutputObserver for RemoteWorkerEventObserver {
    fn observe_output(&self, chunk: TaskOutputChunk) -> Result<()> {
        append_tail_bytes(
            match chunk.stream {
                OutputStream::Stdout => &self.stdout_tail,
                OutputStream::Stderr => &self.stderr_tail,
            },
            &chunk.bytes,
        );

        let kind = match chunk.stream {
            OutputStream::Stdout => "TASK_STDOUT_CHUNK",
            OutputStream::Stderr => "TASK_STDERR_CHUNK",
        };
        if let Err(error) = self.store.append_event(
            &self.idempotency_key,
            self.claim_next_seq(),
            &serde_json::json!({
                "kind": kind,
                "timestamp_ms": unix_epoch_ms(),
                "chunk": String::from_utf8_lossy(&chunk.bytes).into_owned(),
                "chunk_base64": base64::engine::general_purpose::STANDARD.encode(&chunk.bytes),
            })
            .to_string(),
        ) {
            tracing::error!(
                "failed to append {kind} event for submit {}: {error:#}",
                self.idempotency_key
            );
        }
        Ok(())
    }
}

fn append_tail_bytes(buffer: &Mutex<Vec<u8>>, bytes: &[u8]) {
    let Ok(mut guard) = buffer.lock() else {
        return;
    };
    guard.extend_from_slice(bytes);
    if guard.len() > REMOTE_RESULT_TAIL_LIMIT_BYTES {
        let drain_len = guard.len() - REMOTE_RESULT_TAIL_LIMIT_BYTES;
        guard.drain(..drain_len);
    }
}

fn read_tail_buffer(buffer: &Mutex<Vec<u8>>) -> String {
    let Ok(guard) = buffer.lock() else {
        return String::new();
    };
    String::from_utf8_lossy(&guard).into_owned()
}

fn json_tail_value(value: &str) -> serde_json::Value {
    if value.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::json!(value)
    }
}
