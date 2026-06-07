use std::io::Write;

use futures::future;
use futures::io::AllowStdIo;
use magic_wormhole::transfer::request_file;
use magic_wormhole::transit::{Abilities, DEFAULT_RELAY_SERVER, RelayHint};
use magic_wormhole::{MailboxConnection, Wormhole};
use prost::Message;
use tak_proto::{StartWorkspaceWormholeUploadRequest, StartWorkspaceWormholeUploadResponse};

use super::*;

pub(super) fn wormhole_upload_available(upload_id: &str) -> RemoteV1Response {
    protobuf_response(
        200,
        &StartWorkspaceWormholeUploadResponse {
            upload_id: upload_id.to_string(),
            size_bytes: 0,
            complete: false,
        },
    )
}

pub(in crate::daemon::remote) async fn receive_workspace_wormhole_upload(
    context: &RemoteNodeContext,
    path_only: &str,
    body: &[u8],
) -> Result<RemoteV1Response> {
    let Some(upload_id) = super::upload_path_arg(path_only, "/wormhole") else {
        return Ok(error_response(404, "not_found:workspace_wormhole_upload"));
    };
    let request = StartWorkspaceWormholeUploadRequest::decode(body)
        .map_err(|_| anyhow!("invalid workspace wormhole upload protobuf"))?;
    if request.upload_id != upload_id {
        return Ok(error_response(400, "upload_id_mismatch"));
    }
    let metadata = UploadMetadata::from_begin(&tak_proto::BeginWorkspaceUploadRequest {
        task_run_id: String::new(),
        attempt: 0,
        sha256: request.sha256,
        size_bytes: request.size_bytes,
    })?;
    ensure_upload_root(context)?;
    ensure_metadata(context, upload_id, &metadata)?;
    let status = upload_status(context, upload_id, &metadata)?;
    if status.complete {
        return Ok(response(upload_id, metadata.size_bytes, true));
    }
    receive_archive(context, upload_id, &metadata, &request.code).await?;
    Ok(response(upload_id, metadata.size_bytes, true))
}

async fn receive_archive(
    context: &RemoteNodeContext,
    upload_id: &str,
    metadata: &UploadMetadata,
    code: &str,
) -> Result<()> {
    truncate_partial_upload(context, upload_id, 0)?;
    let code = code.parse().context("parse workspace wormhole code")?;
    let mailbox = MailboxConnection::connect(magic_wormhole::transfer::APP_CONFIG, code, false)
        .await
        .context("connect workspace wormhole mailbox")?;
    let wormhole = Wormhole::connect(mailbox)
        .await
        .context("connect workspace wormhole")?;
    let Some(offer) = request_file(
        wormhole,
        relay_hints()?,
        transfer_abilities("TAKD_WORMHOLE_TRANSIT"),
        future::pending::<()>(),
    )
    .await
    .context("request workspace wormhole file")?
    else {
        bail!("workspace wormhole transfer cancelled");
    };
    if offer.file_size() != metadata.size_bytes {
        let _ = offer.reject().await;
        bail!("workspace wormhole size mismatch");
    }
    let path = partial_upload_path(context, upload_id);
    let file = std::fs::File::create(&path)
        .with_context(|| format!("create partial wormhole upload {}", path.display()))?;
    let mut writer = AllowStdIo::new(file);
    offer
        .accept(
            |info| tracing::info!(transit = %info, "workspace wormhole transit established"),
            |_received, _total| {},
            &mut writer,
            future::pending::<()>(),
        )
        .await
        .context("receive workspace archive over wormhole")?;
    writer
        .get_mut()
        .flush()
        .context("flush partial wormhole upload")?;
    let bytes = fs::read(&path).context("read received workspace wormhole upload")?;
    if let Err(err) = ensure_upload_matches(metadata, &bytes) {
        let _ = fs::remove_file(&path);
        return Err(err);
    }
    commit_partial_upload(context, upload_id)
}

fn response(upload_id: &str, size_bytes: u64, complete: bool) -> RemoteV1Response {
    protobuf_response(
        200,
        &StartWorkspaceWormholeUploadResponse {
            upload_id: upload_id.to_string(),
            size_bytes,
            complete,
        },
    )
}

fn relay_hints() -> Result<Vec<RelayHint>> {
    let relay = DEFAULT_RELAY_SERVER
        .parse()
        .context("parse default magic-wormhole relay URL")?;
    Ok(vec![
        RelayHint::from_urls(None, [relay]).context("create default magic-wormhole relay hint")?,
    ])
}

fn transfer_abilities(env_name: &str) -> Abilities {
    match std::env::var(env_name).unwrap_or_default().trim() {
        "relay" => Abilities::FORCE_RELAY,
        _ => Abilities::ALL,
    }
}
