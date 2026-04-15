use super::*;
use prost::Message;
use tak_proto::ErrorResponse;

pub(super) fn resolve_submit_idempotency_key_for_task_run(
    store: &SubmitAttemptStore,
    task_run_id: &str,
    query: Option<&str>,
) -> Result<Option<String>> {
    if let Some(attempt) =
        query_param_u64(query, "attempt").and_then(|value| u32::try_from(value).ok())
    {
        let key = build_submit_idempotency_key(task_run_id, Some(attempt))?;
        return Ok(Some(key));
    }
    store.latest_submit_idempotency_key_for_task_run(task_run_id)
}

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

pub(super) fn execution_root_for_submit_key(idempotency_key: &str) -> PathBuf {
    remote_execution_root_base().join(sanitize_submit_idempotency_key(idempotency_key))
}

pub(super) fn artifact_root_for_submit_key(idempotency_key: &str) -> PathBuf {
    remote_artifact_root_base().join(sanitize_submit_idempotency_key(idempotency_key))
}

pub(super) fn remote_execution_root_base() -> PathBuf {
    std::env::var("TAKD_REMOTE_EXEC_ROOT")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("takd-remote-exec"))
}

pub(super) fn remote_artifact_root_base() -> PathBuf {
    remote_execution_root_base()
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(std::env::temp_dir)
        .join("takd-remote-artifacts")
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

pub(super) fn remote_task_path_arg<'a>(path: &'a str, suffix: &str) -> Option<&'a str> {
    let path = path.strip_prefix("/v1/tasks/")?;
    let task_run_id = path.strip_suffix(suffix)?;
    if task_run_id.is_empty() || task_run_id.contains('/') {
        return None;
    }
    Some(task_run_id)
}

pub(super) fn binary_response(
    status_code: u16,
    content_type: &str,
    body: impl Into<Vec<u8>>,
) -> RemoteV1Response {
    RemoteV1Response {
        status_code,
        content_type: content_type.to_string(),
        body: body.into(),
    }
}

pub(super) fn protobuf_response<M: Message>(status_code: u16, message: &M) -> RemoteV1Response {
    binary_response(
        status_code,
        "application/x-protobuf",
        message.encode_to_vec(),
    )
}

pub(super) fn error_response(status_code: u16, message: &str) -> RemoteV1Response {
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
