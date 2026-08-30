use serde_json::json;

use super::Reply;

pub(super) fn response_bytes(reply: &Reply, request_id: &str) -> Option<Vec<u8>> {
    let value = match reply {
        Reply::Inactive(message) | Reply::SlowDripInactive(message, _, _) => json!({
            "protocol_version": 2,
            "type": "Error",
            "request_id": request_id,
            "message": message,
            "code": "protocol_v2_not_active",
            "retryable": false,
        }),
        Reply::Legacy(message) => json!({
            "type": "Error", "request_id": request_id, "message": message,
        }),
        Reply::Retryable(message) => json!({
            "protocol_version": 2,
            "type": "Error",
            "request_id": request_id,
            "message": message,
            "code": "protocol_v2_not_active",
            "retryable": true,
        }),
        Reply::Success => json!({
            "protocol_version": 2, "type": "RunList", "request_id": request_id, "runs": [],
        }),
        Reply::Raw(bytes) | Reply::RawThenStall(bytes) => return Some(bytes.clone()),
        Reply::Close => return None,
    };
    let mut bytes = serde_json::to_vec(&value).expect("encode fake response");
    bytes.push(b'\n');
    Some(bytes)
}
