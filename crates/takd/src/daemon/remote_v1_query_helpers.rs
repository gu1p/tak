fn resolve_submit_idempotency_key_for_task_run(
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

fn split_path_and_query(path: &str) -> (&str, Option<&str>) {
    match path.split_once('?') {
        Some((path_only, query)) => (path_only, Some(query)),
        None => (path, None),
    }
}

fn query_param_string<'a>(query: Option<&'a str>, key: &str) -> Option<&'a str> {
    let query = query?;
    query.split('&').find_map(|pair| {
        let (name, value) = pair.split_once('=')?;
        if name == key { Some(value) } else { None }
    })
}

fn query_param_u64(query: Option<&str>, key: &str) -> Option<u64> {
    query_param_string(query, key).and_then(|value| value.parse::<u64>().ok())
}

fn execution_root_for_submit_key(idempotency_key: &str) -> PathBuf {
    let base = std::env::var("TAKD_REMOTE_EXEC_ROOT")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("takd-remote-exec"));

    base.join(sanitize_submit_idempotency_key(idempotency_key))
}

fn sanitize_submit_idempotency_key(idempotency_key: &str) -> String {
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

fn remote_task_path_arg<'a>(path: &'a str, suffix: &str) -> Option<&'a str> {
    let path = path.strip_prefix("/v1/tasks/")?;
    let task_run_id = path.strip_suffix(suffix)?;
    if task_run_id.is_empty() || task_run_id.contains('/') {
        return None;
    }
    Some(task_run_id)
}

fn json_response(status_code: u16, body: serde_json::Value) -> RemoteV1Response {
    RemoteV1Response {
        status_code,
        content_type: "application/json".to_string(),
        body: body.to_string(),
    }
}

/// Returns the current Unix epoch timestamp in milliseconds.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn unix_epoch_ms() -> i64 {
    let now = SystemTime::now();
    let duration = now
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before UNIX_EPOCH");
    i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
}
