use super::*;

pub(super) fn handle_remote_outputs_route(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    method: &str,
    path_only: &str,
    query: Option<&str>,
    headers: &[(String, String)],
) -> Result<Option<RemoteV1Response>> {
    let Some(task_run_id) = remote_task_path_arg(path_only, "/outputs") else {
        return Ok(None);
    };
    if method != "GET" {
        return Ok(None);
    }

    let key = resolve_submit_idempotency_key_for_task_run(store, task_run_id, query)?;
    let Some(key) = key else {
        return Ok(Some(error_response(404, "task_not_found")));
    };
    let Some(raw_path) = query_param_string(query, "path") else {
        return Ok(Some(error_response(400, "missing_output_path")));
    };
    let normalized = match normalize_path_ref("workspace", &raw_path) {
        Ok(path_ref) if path_ref.path != "." => path_ref.path,
        _ => return Ok(Some(error_response(400, "invalid_output_path"))),
    };
    let execution_root_base = store
        .execution_root_base_for_submit(&key)?
        .unwrap_or_else(|| remote_execution_root_base(context));
    let artifact_root = artifact_root_for_submit_key_at_base(&key, &execution_root_base);
    let output_path = if artifact_root.exists() {
        artifact_root.join(&normalized)
    } else {
        execution_root_for_submit_key_at_base(&key, &execution_root_base).join(&normalized)
    };
    output_file_response(&output_path, headers)
        .map(Some)
        .or_else(|err| match err.downcast_ref::<OutputNotFound>() {
            Some(_) => Ok(Some(error_response(404, "output_not_found"))),
            None => Err(err),
        })
}

#[derive(Debug)]
struct OutputNotFound;

impl std::fmt::Display for OutputNotFound {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("output not found")
    }
}

impl std::error::Error for OutputNotFound {}

fn output_file_response(path: &Path, headers: &[(String, String)]) -> Result<RemoteV1Response> {
    let metadata = fs::metadata(path).map_err(|err| match err.kind() {
        std::io::ErrorKind::NotFound => anyhow::Error::new(OutputNotFound),
        _ => anyhow::Error::new(err).context(format!("failed to stat output {}", path.display())),
    })?;
    let file_len = metadata.len();
    let Some(range) = byte_range(headers, file_len) else {
        let bytes = fs::read(path).map_err(|err| match err.kind() {
            std::io::ErrorKind::NotFound => anyhow::Error::new(OutputNotFound),
            _ => {
                anyhow::Error::new(err).context(format!("failed to read output {}", path.display()))
            }
        })?;
        return Ok(binary_response(200, "application/octet-stream", bytes));
    };
    if range.start >= file_len || range.end < range.start {
        return Ok(binary_response_with_headers(
            416,
            "application/octet-stream",
            vec![("Content-Range".to_string(), format!("bytes */{file_len}"))],
            Vec::new(),
        ));
    };
    let end = range.end.min(file_len.saturating_sub(1));
    let body = read_file_range(path, range.start, end)?;
    Ok(binary_response_with_headers(
        206,
        "application/octet-stream",
        vec![(
            "Content-Range".to_string(),
            format!("bytes {}-{}/{}", range.start, end, file_len),
        )],
        body,
    ))
}

struct OutputByteRange {
    start: u64,
    end: u64,
}

fn byte_range(headers: &[(String, String)], file_len: u64) -> Option<OutputByteRange> {
    let value = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("range"))?
        .1
        .trim();
    let range = value.strip_prefix("bytes=")?;
    let (start, end) = range.split_once('-')?;
    let start = start.trim().parse::<u64>().ok()?;
    let end = if end.trim().is_empty() {
        file_len.saturating_sub(1)
    } else {
        end.trim().parse::<u64>().ok()?
    };
    Some(OutputByteRange { start, end })
}

fn read_file_range(path: &Path, start: u64, end: u64) -> Result<Vec<u8>> {
    use std::io::{Read, Seek, SeekFrom};

    let mut file = fs::File::open(path)
        .with_context(|| format!("failed to open output {}", path.display()))?;
    file.seek(SeekFrom::Start(start))
        .with_context(|| format!("failed to seek output {}", path.display()))?;
    let len = end.saturating_sub(start).saturating_add(1);
    let mut limited = file.take(len);
    let mut body = Vec::with_capacity(usize::try_from(len).unwrap_or(0));
    limited
        .read_to_end(&mut body)
        .with_context(|| format!("failed to read output range {}", path.display()))?;
    Ok(body)
}
