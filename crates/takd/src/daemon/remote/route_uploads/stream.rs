use super::*;
use http_body_util::BodyExt;
use tak_proto::{AppendWorkspaceUploadResponse, BeginWorkspaceUploadResponse};
use tokio::io::AsyncWriteExt;

pub(in crate::daemon::remote) async fn stream_workspace_upload(
    context: &RemoteNodeContext,
    path_only: &str,
    query: Option<&str>,
    headers: &hyper::HeaderMap,
    body: hyper::body::Incoming,
) -> Result<RemoteV1Response> {
    let Some(upload_id) = super::upload_path_arg(path_only, "/stream") else {
        return Ok(error_response(404, "not_found:workspace_upload_stream"));
    };
    let metadata = metadata_from_headers(headers)?;
    ensure_upload_root(context)?;
    ensure_metadata(context, upload_id, &metadata)?;
    let offset = query_param_u64(query, "offset").unwrap_or(0);
    let status = upload_status(context, upload_id, &metadata)?;
    if status.complete {
        drain_body(body).await?;
        return Ok(stream_response(metadata.size_bytes, true));
    }
    if offset != status.offset {
        drain_body(body).await?;
        return Ok(stream_response(status.offset, false));
    }
    receive_body(context, upload_id, &metadata, offset, body).await
}

pub(super) fn workspace_upload_status(
    context: &RemoteNodeContext,
    upload_id: &str,
) -> Result<RemoteV1Response> {
    let metadata = read_metadata(context, upload_id)?;
    let status = upload_status(context, upload_id, &metadata)?;
    Ok(protobuf_response(
        200,
        &BeginWorkspaceUploadResponse {
            upload_id: upload_id.to_string(),
            offset: status.offset,
            complete: status.complete,
        },
    ))
}

async fn receive_body(
    context: &RemoteNodeContext,
    upload_id: &str,
    metadata: &UploadMetadata,
    offset: u64,
    body: hyper::body::Incoming,
) -> Result<RemoteV1Response> {
    truncate_partial_upload(context, upload_id, offset)?;
    let mut hasher = hash_partial_prefix(context, upload_id, offset)?;
    let mut received = offset;
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(partial_upload_path(context, upload_id))
        .await
        .context("open partial upload for streaming")?;
    let mut body = body;
    while let Some(frame) = body.frame().await {
        let frame = frame.map_err(|_| anyhow!("truncated_body"))?;
        let Some(data) = frame.data_ref() else {
            continue;
        };
        if received.saturating_add(data.len() as u64) > metadata.size_bytes {
            return Ok(error_response(400, "upload_exceeds_declared_size"));
        }
        file.write_all(data)
            .await
            .context("write streamed upload")?;
        hasher.update(data);
        received += data.len() as u64;
    }
    file.flush().await.context("flush streamed upload")?;
    if received != metadata.size_bytes {
        return Ok(stream_response(received, false));
    }
    let digest = format!("{:x}", hasher.finalize());
    if digest != metadata.sha256 {
        let _ = std::fs::remove_file(partial_upload_path(context, upload_id));
        return Ok(error_response(400, "digest_mismatch"));
    }
    commit_partial_upload(context, upload_id)?;
    tracing::info!(
        upload_id,
        size_bytes = metadata.size_bytes,
        "workspace upload stream committed"
    );
    Ok(stream_response(received, true))
}

async fn drain_body(mut body: hyper::body::Incoming) -> Result<()> {
    while let Some(frame) = body.frame().await {
        frame.map_err(|_| anyhow!("truncated_body"))?;
    }
    Ok(())
}

fn metadata_from_headers(headers: &hyper::HeaderMap) -> Result<UploadMetadata> {
    Ok(UploadMetadata {
        sha256: required_header(headers, "x-tak-upload-sha256")?
            .trim()
            .to_ascii_lowercase(),
        size_bytes: required_header(headers, "x-tak-upload-size")?
            .parse()
            .context("parse upload size")?,
    })
    .and_then(|metadata| {
        UploadMetadata::from_begin(&tak_proto::BeginWorkspaceUploadRequest {
            task_run_id: String::new(),
            attempt: 0,
            sha256: metadata.sha256,
            size_bytes: metadata.size_bytes,
        })
    })
}

fn required_header(headers: &hyper::HeaderMap, name: &str) -> Result<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
        .ok_or_else(|| anyhow!("missing upload header {name}"))
}

fn stream_response(offset: u64, complete: bool) -> RemoteV1Response {
    protobuf_response(200, &AppendWorkspaceUploadResponse { offset, complete })
}
