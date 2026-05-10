use std::path::{Path, PathBuf};

use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::task::JoinHandle;

use super::RecordingEvents;

pub struct RecordingLeaseServer {
    pub socket_path: PathBuf,
    handle: JoinHandle<()>,
}

impl RecordingLeaseServer {
    pub async fn spawn(socket_path: &Path, events: RecordingEvents) -> Self {
        let _ = std::fs::remove_file(socket_path);
        let listener = UnixListener::bind(socket_path).expect("bind recording lease socket");
        let socket_path = socket_path.to_path_buf();
        let handle = tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let events = events.clone();
                tokio::spawn(async move {
                    let (reader, mut writer) = stream.into_split();
                    let mut reader = BufReader::new(reader);
                    let mut line = String::new();
                    if reader.read_line(&mut line).await.unwrap_or(0) == 0 {
                        return;
                    }
                    let response = response_for_request(line.trim_end(), &events);
                    let _ = writer.write_all(format!("{response}\n").as_bytes()).await;
                });
            }
        });
        Self {
            socket_path,
            handle,
        }
    }
}

impl Drop for RecordingLeaseServer {
    fn drop(&mut self) {
        self.handle.abort();
        let _ = std::fs::remove_file(&self.socket_path);
    }
}

fn response_for_request(line: &str, events: &RecordingEvents) -> Value {
    let request: Value = serde_json::from_str(line).expect("decode lease request");
    let request_id = request
        .get("request_id")
        .and_then(Value::as_str)
        .unwrap_or("request");
    match request.get("type").and_then(Value::as_str) {
        Some("AcquireLease") => {
            events.record(format!("lease_acquire:{}", need_names(&request)));
            serde_json::json!({
                "type": "LeaseGranted",
                "request_id": request_id,
                "lease": {
                    "lease_id": "lease-1",
                    "ttl_ms": 30000,
                    "renew_after_ms": 15000
                }
            })
        }
        Some("ReleaseLease") => {
            events.record("lease_release");
            serde_json::json!({ "type": "LeaseReleased", "request_id": request_id })
        }
        other => serde_json::json!({
            "type": "Error",
            "request_id": request_id,
            "message": format!("unexpected request type {other:?}")
        }),
    }
}

fn need_names(request: &Value) -> String {
    request
        .get("needs")
        .and_then(Value::as_array)
        .map(|needs| {
            needs
                .iter()
                .filter_map(|need| need.get("name").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join(",")
        })
        .unwrap_or_default()
}
