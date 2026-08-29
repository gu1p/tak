use serde::Deserialize;

use super::identifier::is_valid_identifier;
use super::{Operation, PROTOCOL_VERSION, Request};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireRequest {
    protocol_version: u64,
    request_id: String,
    operation: WireOperation,
}

#[derive(Deserialize)]
#[serde(tag = "type", deny_unknown_fields)]
enum WireOperation {
    ListRuns {},
    GetRun { run_id: String },
    AttachRun { run_id: String, after_event: u64 },
    CancelRun { run_id: String },
}

pub(super) fn decode_strict(raw: &str) -> Option<Request> {
    let wire = serde_json::from_str::<WireRequest>(raw).ok()?;
    if wire.protocol_version != PROTOCOL_VERSION || !is_valid_identifier(&wire.request_id) {
        return None;
    }
    let operation = match wire.operation {
        WireOperation::ListRuns {} => Operation::ListRuns {},
        WireOperation::GetRun { run_id } if is_valid_identifier(&run_id) => {
            Operation::GetRun { run_id }
        }
        WireOperation::AttachRun {
            run_id,
            after_event,
        } if is_valid_identifier(&run_id) => Operation::AttachRun {
            run_id,
            after_event,
        },
        WireOperation::CancelRun { run_id } if is_valid_identifier(&run_id) => {
            Operation::CancelRun { run_id }
        }
        WireOperation::GetRun { .. }
        | WireOperation::AttachRun { .. }
        | WireOperation::CancelRun { .. } => return None,
    };
    Some(Request {
        request_id: wire.request_id,
        operation,
    })
}
