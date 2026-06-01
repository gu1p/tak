use serde_json::Value;

use crate::support::RecordingEvents;

#[derive(Clone, Copy)]
pub struct RecordingLeaseConfig {
    pub ttl_ms: u64,
    pub renew_after_ms: u64,
}

impl Default for RecordingLeaseConfig {
    fn default() -> Self {
        Self {
            ttl_ms: 30_000,
            renew_after_ms: 15_000,
        }
    }
}

pub fn for_request(line: &str, events: &RecordingEvents, config: RecordingLeaseConfig) -> Value {
    let request: Value = serde_json::from_str(line).expect("decode lease request");
    let request_id = request
        .get("request_id")
        .and_then(Value::as_str)
        .unwrap_or("request");
    match request.get("type").and_then(Value::as_str) {
        Some("AcquireLease") => lease_granted(request_id, &request, events, config),
        Some("RenewLease") => lease_renewed(request_id, events, config),
        Some("ReleaseLease") => lease_released(request_id, events),
        other => serde_json::json!({
            "type": "Error",
            "request_id": request_id,
            "message": format!("unexpected request type {other:?}")
        }),
    }
}

fn lease_granted(
    request_id: &str,
    request: &Value,
    events: &RecordingEvents,
    config: RecordingLeaseConfig,
) -> Value {
    events.record(format!("lease_acquire:{}", need_names(request)));
    serde_json::json!({
        "type": "LeaseGranted",
        "request_id": request_id,
        "lease": {
            "lease_id": "lease-1",
            "ttl_ms": config.ttl_ms,
            "renew_after_ms": config.renew_after_ms
        }
    })
}

fn lease_renewed(
    request_id: &str,
    events: &RecordingEvents,
    config: RecordingLeaseConfig,
) -> Value {
    events.record("lease_renew");
    serde_json::json!({
        "type": "LeaseRenewed",
        "request_id": request_id,
        "ttl_ms": config.ttl_ms
    })
}

fn lease_released(request_id: &str, events: &RecordingEvents) -> Value {
    events.record("lease_release");
    serde_json::json!({ "type": "LeaseReleased", "request_id": request_id })
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
