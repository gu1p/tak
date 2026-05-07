use anyhow::{Result, bail};
use tak_core::model::RemoteTransportKind;
use tak_exec::{endpoint_host_port, endpoint_socket_addr};

use super::remote_inventory::RemoteRecord;
use super::remote_probe_support::http::send_http_get;
use super::remote_probe_support::transport::{TorOnionRetryTexts, connect_remote, retry_tor_onion};
use super::remote_probe_support::{ProbeAttemptError, RemoteStream};

pub(super) async fn get_remote_bytes(remote: &RemoteRecord, path: &str) -> Result<(u16, Vec<u8>)> {
    let kind = remote_transport_kind(&remote.transport)?;
    let (host, port) = endpoint_host_port(&remote.base_url)?;
    let authority = endpoint_socket_addr(&remote.base_url)?;
    if kind != RemoteTransportKind::Tor || !host.ends_with(".onion") {
        let stream = connect_remote(&host, port, kind).await?;
        return get_once(
            stream,
            &authority,
            &remote.bearer_token,
            &remote.base_url,
            path,
        )
        .await
        .map_err(ProbeAttemptError::into_anyhow);
    }

    retry_tor_onion(
        &remote.base_url,
        &host,
        port,
        TorOnionRetryTexts {
            build_config: "build tor remote inspection client config",
            bootstrap: "bootstrap tor remote inspection client",
            connect: "connect remote inspection",
            timeout_tail: " while inspecting remote node",
            no_retryable_error: "remote inspection failed without a retryable error",
        },
        |stream| {
            get_once(
                stream,
                &authority,
                &remote.bearer_token,
                &remote.base_url,
                path,
            )
        },
    )
    .await
}

fn remote_transport_kind(transport: &str) -> Result<RemoteTransportKind> {
    match transport {
        "direct" => Ok(RemoteTransportKind::Direct),
        "tor" => Ok(RemoteTransportKind::Tor),
        _ => bail!("unsupported remote transport `{transport}`"),
    }
}

async fn get_once(
    stream: RemoteStream,
    authority: &str,
    bearer_token: &str,
    base_url: &str,
    path: &str,
) -> std::result::Result<(u16, Vec<u8>), ProbeAttemptError> {
    send_http_get(
        stream,
        authority,
        path,
        bearer_token,
        base_url,
        "write remote inspection request",
    )
    .await
}
