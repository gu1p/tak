use base64::Engine;
use std::sync::Mutex;

const REMOTE_RESULT_TAIL_LIMIT_BYTES: usize = 4096;

#[derive(Clone)]
struct RemoteWorkerEventObserver {
    events: RemoteWorkerEventWriter,
    stdout_tail: Arc<Mutex<Vec<u8>>>,
    stderr_tail: Arc<Mutex<Vec<u8>>>,
}

impl RemoteWorkerEventObserver {
    fn new(events: RemoteWorkerEventWriter) -> Self {
        Self {
            events,
            stdout_tail: Arc::new(Mutex::new(Vec::new())),
            stderr_tail: Arc::new(Mutex::new(Vec::new())),
        }
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
        if let Err(error) = self.events.append(serde_json::json!({
                "kind": kind,
                "timestamp_ms": unix_epoch_ms(),
                "chunk_base64": base64::engine::general_purpose::STANDARD.encode(&chunk.bytes),
            })) {
            tracing::error!("failed to append {kind} event: {error:#}");
        }
        Ok(())
    }

    fn observe_status(&self, event: TaskStatusEvent) -> Result<()> {
        append_tail_bytes(&self.stderr_tail, event.message.as_bytes());
        append_tail_bytes(&self.stderr_tail, b"\n");
        self.events.append(serde_json::json!({
                "kind": "TASK_STATUS",
                "timestamp_ms": unix_epoch_ms(),
                "message": event.message,
            }))?;
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
