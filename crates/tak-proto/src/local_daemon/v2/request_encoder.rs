use serde::Serialize;

use super::identifier::is_valid_identifier;
use super::{Operation, PROTOCOL_VERSION, Request, RequestEncodeError};

#[derive(Serialize)]
struct WireRequest<'a> {
    protocol_version: u64,
    request_id: &'a str,
    operation: WireOperation<'a>,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum WireOperation<'a> {
    ListRuns {},
    GetRun { run_id: &'a str },
    AttachRun { run_id: &'a str, after_event: u64 },
    CancelRun { run_id: &'a str },
    GetOutputManifest { run_id: &'a str },
}

/// Encodes one strict protocol-v2 request without its transport delimiter.
///
/// ```rust
/// use tak_proto::local_daemon::v2::{Operation, Request, encode_request};
///
/// let request = Request {
///     request_id: "list".into(),
///     operation: Operation::ListRuns {},
/// };
/// let encoded = encode_request(&request)?;
/// assert!(encoded.contains(r#""protocol_version":2"#));
/// assert!(!encoded.contains('\n'));
/// # Ok::<(), tak_proto::local_daemon::v2::RequestEncodeError>(())
/// ```
pub fn encode_request(request: &Request) -> Result<String, RequestEncodeError> {
    if !is_valid_identifier(&request.request_id) {
        return Err(RequestEncodeError::RequestIdInvalid);
    }
    let operation = encode_operation(&request.operation)?;
    serde_json::to_string(&WireRequest {
        protocol_version: PROTOCOL_VERSION,
        request_id: &request.request_id,
        operation,
    })
    .map_err(|_| RequestEncodeError::EncodingFailed)
}

fn encode_operation(operation: &Operation) -> Result<WireOperation<'_>, RequestEncodeError> {
    let encoded = match operation {
        Operation::ListRuns {} => WireOperation::ListRuns {},
        Operation::GetRun { run_id } => WireOperation::GetRun {
            run_id: valid_run_id(run_id)?,
        },
        Operation::AttachRun {
            run_id,
            after_event,
        } => WireOperation::AttachRun {
            run_id: valid_run_id(run_id)?,
            after_event: *after_event,
        },
        Operation::CancelRun { run_id } => WireOperation::CancelRun {
            run_id: valid_run_id(run_id)?,
        },
        Operation::GetOutputManifest { run_id } => WireOperation::GetOutputManifest {
            run_id: valid_run_id(run_id)?,
        },
    };
    Ok(encoded)
}

fn valid_run_id(run_id: &str) -> Result<&str, RequestEncodeError> {
    is_valid_identifier(run_id)
        .then_some(run_id)
        .ok_or(RequestEncodeError::RunIdInvalid)
}
