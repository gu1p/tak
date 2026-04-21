use super::remote_probe_support::http::send_http_get;
use super::remote_probe_support::transport::{TorOnionRetryTexts, connect_remote, retry_tor_onion};
use super::remote_probe_support::{ProbeAttemptError, RemoteStream};
use anyhow::{Context, Result, anyhow, bail};
use prost::Message;
use tak_core::model::RemoteTransportKind;
use tak_exec::{endpoint_host_port, endpoint_socket_addr};
use tak_proto::NodeInfo;

pub(super) async fn probe_node(
    base_url: &str,
    transport: &str,
    bearer_token: &str,
) -> Result<NodeInfo> {
    let kind = match transport {
        "direct" => RemoteTransportKind::Direct,
        "tor" => RemoteTransportKind::Tor,
        _ => bail!("unsupported remote transport `{transport}`"),
    };
    let (host, port) = endpoint_host_port(base_url)?;
    let authority = endpoint_socket_addr(base_url)?;
    if kind != RemoteTransportKind::Tor || !host.ends_with(".onion") {
        let stream = connect_remote(&host, port, kind).await?;
        return probe_once(stream, &authority, bearer_token, base_url)
            .await
            .map_err(ProbeAttemptError::into_anyhow);
    }

    retry_tor_onion(
        base_url,
        &host,
        port,
        TorOnionRetryTexts {
            build_config: "build tor node probe client config",
            bootstrap: "bootstrap tor node probe client",
            connect: "connect node probe",
            timeout_tail: "; a freshly started takd hidden service may still be propagating",
            no_retryable_error: "node probe failed without a retryable error",
        },
        |stream| probe_once(stream, &authority, bearer_token, base_url),
    )
    .await
}

async fn probe_once(
    stream: RemoteStream,
    authority: &str,
    bearer_token: &str,
    base_url: &str,
) -> std::result::Result<NodeInfo, ProbeAttemptError> {
    let (status, body) = send_http_get(
        stream,
        authority,
        "/v1/node/info",
        bearer_token,
        base_url,
        "write node probe",
    )
    .await?;
    if status != 200 {
        return Err(ProbeAttemptError::final_error(anyhow!(
            "node probe failed with HTTP {status}"
        )));
    }
    NodeInfo::decode(body.as_slice())
        .context("decode node info protobuf")
        .map_err(ProbeAttemptError::final_error)
}

mod remote_probe_connection_cleanup_tests;
#[cfg(test)]
mod remote_probe_tests;
