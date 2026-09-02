use super::*;
use prost::Message;
use tak_proto::ErrorResponse;

pub(super) fn split_path_and_query(path: &str) -> (&str, Option<&str>) {
    match path.split_once('?') {
        Some((path_only, query)) => (path_only, Some(query)),
        None => (path, None),
    }
}

pub(super) fn query_param_string(query: Option<&str>, key: &str) -> Option<String> {
    let query = query?;
    url::form_urlencoded::parse(query.as_bytes())
        .find_map(|(name, value)| (name == key).then(|| value.into_owned()))
}

pub(super) fn query_param_u64(query: Option<&str>, key: &str) -> Option<u64> {
    query_param_string(query, key).and_then(|value| value.parse::<u64>().ok())
}

pub(super) fn sanitize_submit_idempotency_key(idempotency_key: &str) -> String {
    idempotency_key
        .chars()
        .map(|value| {
            if value.is_ascii_alphanumeric() || matches!(value, '.' | '-' | '_') {
                value
            } else {
                '_'
            }
        })
        .collect()
}

pub(super) fn binary_response(
    status_code: u16,
    content_type: &str,
    body: impl Into<Vec<u8>>,
) -> WorkerHttpResponse {
    WorkerHttpResponse {
        status_code,
        content_type: content_type.to_string(),
        headers: Vec::new(),
        body: body.into(),
    }
}

pub(super) fn text_response(status_code: u16, body: impl Into<String>) -> WorkerHttpResponse {
    binary_response(
        status_code,
        "text/plain; charset=utf-8",
        body.into().into_bytes(),
    )
}

pub(super) fn protobuf_response<M: Message>(status_code: u16, message: &M) -> WorkerHttpResponse {
    binary_response(
        status_code,
        "application/x-protobuf",
        message.encode_to_vec(),
    )
}

pub(super) fn error_response(status_code: u16, message: &str) -> WorkerHttpResponse {
    protobuf_response(
        status_code,
        &ErrorResponse {
            message: message.to_string(),
        },
    )
}

/// Returns the current Unix epoch timestamp in milliseconds.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub(super) fn unix_epoch_ms() -> i64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    i64::try_from(millis).unwrap_or(i64::MAX)
}
