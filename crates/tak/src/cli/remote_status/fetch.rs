use anyhow::{Context, Result, anyhow, bail};
use arti_client::TorClient;
use futures::future::join_all;
use std::time::Instant;
use tak_core::model::RemoteTransportKind;
use tak_exec::default_client_tor_config;
use tak_proto::NodeStatusResponse;
use tokio::io::AsyncRead;
use tokio::net::TcpStream;
use tokio::time::sleep;

use crate::cli::remote_probe_support::{
    ProbeAttemptError, test_tor_onion_dial_addr, tor_probe_retry_policy,
};

use super::http::{endpoint_host_port, endpoint_socket_addr, fetch_status_once};
use super::{RemoteRecord, RemoteStatusResult};

pub(super) async fn fetch_snapshot(remotes: &[RemoteRecord]) -> Vec<RemoteStatusResult> {
    let mut results = join_all(remotes.iter().map(fetch_remote_status_result)).await;
    results.sort_unstable_by(|left, right| left.remote.node_id.cmp(&right.remote.node_id));
    results
}

async fn fetch_remote_status_result(remote: &RemoteRecord) -> RemoteStatusResult {
    let remote = remote.clone();
    match fetch_node_status(&remote.base_url, &remote.transport, &remote.bearer_token).await {
        Ok(status) => RemoteStatusResult {
            remote,
            status: Some(status),
            error: None,
        },
        Err(err) => RemoteStatusResult {
            remote,
            status: None,
            error: Some(err.to_string()),
        },
    }
}

async fn fetch_node_status(
    base_url: &str,
    transport: &str,
    bearer_token: &str,
) -> Result<NodeStatusResponse> {
    let kind = match transport {
        "direct" => RemoteTransportKind::Direct,
        "tor" => RemoteTransportKind::Tor,
        _ => bail!("unsupported remote transport `{transport}`"),
    };
    let (host, port) = endpoint_host_port(base_url)?;
    let authority = endpoint_socket_addr(base_url)?;
    if kind != RemoteTransportKind::Tor || !host.ends_with(".onion") {
        let stream = connect(base_url, kind).await?;
        return fetch_status_once(stream, &authority, bearer_token, base_url)
            .await
            .map_err(ProbeAttemptError::into_anyhow);
    }

    let retry_policy = tor_probe_retry_policy();
    let deadline = Instant::now() + retry_policy.timeout;
    let test_dial_addr = test_tor_onion_dial_addr();
    let mut tor_client = None;
    let mut last_error = anyhow!("node status failed without a retryable error");
    loop {
        if test_dial_addr.is_none() && tor_client.is_none() {
            let config =
                default_client_tor_config().context("build tor node status client config")?;
            match TorClient::create_bootstrapped(config)
                .await
                .context("bootstrap tor node status client")
            {
                Ok(client) => tor_client = Some(client),
                Err(err) => {
                    last_error = ProbeAttemptError::retryable(err).into_anyhow();
                    if Instant::now() >= deadline {
                        break;
                    }
                    sleep(retry_policy.backoff).await;
                    continue;
                }
            }
        }

        let stream = if let Some(test_dial_addr) = test_dial_addr.as_deref() {
            TcpStream::connect(test_dial_addr)
                .await
                .context("connect node status")
                .map(|stream| Box::new(stream) as RemoteStream)
                .map_err(ProbeAttemptError::retryable)
        } else {
            tor_client
                .as_ref()
                .expect("tor client should be initialized before connect")
                .connect((host.as_str(), port))
                .await
                .context("connect node status")
                .map(|stream| Box::new(stream) as RemoteStream)
                .map_err(ProbeAttemptError::retryable)
        };

        match stream {
            Ok(stream) => match fetch_status_once(stream, &authority, bearer_token, base_url).await
            {
                Ok(status) => return Ok(status),
                Err(err) if err.is_retryable() => last_error = err.into_anyhow(),
                Err(err) => return Err(err.into_anyhow()),
            },
            Err(err) => last_error = err.into_anyhow(),
        }

        if Instant::now() >= deadline {
            break;
        }
        sleep(retry_policy.backoff).await;
    }

    Err(last_error).context(format!("Tor onion service at {base_url} did not become reachable within {}ms while requesting node status", retry_policy.timeout.as_millis()))
}

trait RemoteIo: AsyncRead + tokio::io::AsyncWrite {}
impl<T> RemoteIo for T where T: AsyncRead + tokio::io::AsyncWrite + ?Sized {}
type RemoteStream = Box<dyn RemoteIo + Unpin + Send>;

async fn connect(endpoint: &str, kind: RemoteTransportKind) -> Result<RemoteStream> {
    let (host, port) = endpoint_host_port(endpoint)?;
    if kind == RemoteTransportKind::Direct || !host.ends_with(".onion") {
        return Ok(Box::new(
            TcpStream::connect(format!("{host}:{port}")).await?,
        ));
    }
    if let Some(test_dial_addr) = test_tor_onion_dial_addr() {
        return Ok(Box::new(TcpStream::connect(test_dial_addr).await?));
    }
    let config = default_client_tor_config()?;
    Ok(Box::new(
        TorClient::create_bootstrapped(config)
            .await?
            .connect((host.as_str(), port))
            .await?,
    ))
}
