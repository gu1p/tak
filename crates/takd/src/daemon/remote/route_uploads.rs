use super::*;
use prost::Message;
use tak_proto::{
    AppendWorkspaceUploadResponse, BeginWorkspaceUploadRequest, BeginWorkspaceUploadResponse,
    FinishWorkspaceUploadResponse,
};

mod storage;
mod stream;

pub(super) use storage::resolve_workspace_upload_zip;
use storage::{
    UploadMetadata, append_chunk, commit_partial_upload, completed_upload_path,
    current_upload_offset, ensure_metadata, ensure_upload_matches, ensure_upload_root,
    ensure_valid_upload_id, hash_partial_prefix, partial_upload_path, read_metadata,
    truncate_partial_upload, upload_status, write_metadata,
};
pub(super) use stream::stream_workspace_upload;
use stream::workspace_upload_status;

pub(super) fn handle_workspace_upload_route(
    context: &RemoteNodeContext,
    method: &str,
    path_only: &str,
    query: Option<&str>,
    body: Option<&[u8]>,
) -> Result<Option<RemoteV1Response>> {
    if method == "POST" && path_only == "/v2/workspaces/uploads/begin" {
        return Ok(Some(begin_workspace_upload(context, body)?));
    }
    if method == "GET"
        && let Some(upload_id) = upload_path_arg(path_only, "")
    {
        return Ok(Some(workspace_upload_status(context, upload_id)?));
    }
    let Some(upload_id) = upload_path_arg(path_only, "") else {
        let Some(upload_id) = upload_path_arg(path_only, "/finish") else {
            return Ok(None);
        };
        if method != "POST" {
            return Ok(None);
        }
        return Ok(Some(finish_workspace_upload(context, upload_id)?));
    };
    if method != "PATCH" {
        return Ok(None);
    }
    Ok(Some(append_workspace_upload(
        context,
        upload_id,
        query,
        body.unwrap_or_default(),
    )?))
}

fn begin_workspace_upload(
    context: &RemoteNodeContext,
    body: Option<&[u8]>,
) -> Result<RemoteV1Response> {
    let Some(body) = body else {
        return Ok(error_response(400, "missing_body"));
    };
    let request = BeginWorkspaceUploadRequest::decode(body)
        .map_err(|_| anyhow!("invalid workspace upload begin protobuf"))?;
    let metadata = UploadMetadata::from_begin(&request)?;
    let upload_id = upload_id(&request, &metadata);
    ensure_upload_root(context)?;
    write_metadata(context, &upload_id, &metadata)?;
    let status = upload_status(context, &upload_id, &metadata)?;
    Ok(protobuf_response(
        200,
        &BeginWorkspaceUploadResponse {
            upload_id,
            offset: status.offset,
            complete: status.complete,
        },
    ))
}

fn append_workspace_upload(
    context: &RemoteNodeContext,
    upload_id: &str,
    query: Option<&str>,
    chunk: &[u8],
) -> Result<RemoteV1Response> {
    let Some(offset) = query_param_u64(query, "offset") else {
        return Ok(error_response(400, "missing_offset"));
    };
    let metadata = read_metadata(context, upload_id)?;
    let current = current_upload_offset(context, upload_id)?;
    if offset != current {
        return Ok(protobuf_response(
            409,
            &AppendWorkspaceUploadResponse {
                offset: current,
                complete: current == metadata.size_bytes,
            },
        ));
    }
    if offset.saturating_add(chunk.len() as u64) > metadata.size_bytes {
        return Ok(error_response(400, "chunk_exceeds_upload_size"));
    }
    append_chunk(context, upload_id, chunk)?;
    let next = offset + chunk.len() as u64;
    Ok(protobuf_response(
        200,
        &AppendWorkspaceUploadResponse {
            offset: next,
            complete: next == metadata.size_bytes,
        },
    ))
}

fn finish_workspace_upload(
    context: &RemoteNodeContext,
    upload_id: &str,
) -> Result<RemoteV1Response> {
    let metadata = read_metadata(context, upload_id)?;
    if let Ok(bytes) = fs::read(completed_upload_path(context, upload_id)) {
        ensure_upload_matches(&metadata, &bytes)?;
        return Ok(finished_upload_response(upload_id, metadata.size_bytes));
    }
    let part_path = partial_upload_path(context, upload_id);
    let bytes = match fs::read(&part_path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(err) => return Err(err).with_context(|| format!("read upload {upload_id}")),
    };
    if bytes.len() as u64 != metadata.size_bytes {
        return Ok(protobuf_response(
            409,
            &AppendWorkspaceUploadResponse {
                offset: bytes.len() as u64,
                complete: false,
            },
        ));
    }
    if let Err(err) = ensure_upload_matches(&metadata, &bytes) {
        let _ = fs::remove_file(&part_path);
        return Ok(error_response(400, &format!("digest_mismatch:{err:#}")));
    }
    if part_path.exists() {
        fs::rename(&part_path, completed_upload_path(context, upload_id))
            .with_context(|| format!("complete upload {upload_id}"))?;
    } else {
        fs::write(completed_upload_path(context, upload_id), &bytes)
            .with_context(|| format!("complete upload {upload_id}"))?;
    }
    Ok(finished_upload_response(upload_id, metadata.size_bytes))
}

fn finished_upload_response(upload_id: &str, size_bytes: u64) -> RemoteV1Response {
    protobuf_response(
        200,
        &FinishWorkspaceUploadResponse {
            upload_id: upload_id.to_string(),
            size_bytes,
            complete: true,
        },
    )
}

fn upload_path_arg<'a>(path: &'a str, suffix: &str) -> Option<&'a str> {
    let upload_id = path.strip_prefix("/v2/workspaces/uploads/")?;
    let upload_id = upload_id.strip_suffix(suffix)?;
    if ensure_valid_upload_id(upload_id).is_err() {
        return None;
    }
    Some(upload_id)
}

fn upload_id(request: &BeginWorkspaceUploadRequest, metadata: &UploadMetadata) -> String {
    sanitize_submit_idempotency_key(&format!(
        "{}-{}-{}",
        request.task_run_id, request.attempt, metadata.sha256
    ))
}
