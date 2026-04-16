use super::remote_probe_support::{
    AbortOnDrop, ProbeAttemptError, test_tor_onion_dial_addr, tor_probe_retry_policy,
};
use anyhow::{Context, Result, anyhow, bail};
use arti_client::TorClient;
use bytes::Bytes;
use http_body_util::{BodyExt, Empty};
use hyper::Request;
use prost::Message;
use std::time::Instant;
use tak_core::model::RemoteTransportKind;
use tak_exec::{
    default_client_tor_config, endpoint_host_port as shared_endpoint_host_port,
    endpoint_socket_addr as shared_endpoint_socket_addr, socket_addr_from_host_port,
};
use tak_proto::NodeInfo;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio::time::sleep;

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
        let stream = connect(base_url, kind).await?;
        return probe_once(stream, &authority, bearer_token, base_url)
            .await
            .map_err(ProbeAttemptError::into_anyhow);
    }

    let retry_policy = tor_probe_retry_policy();
    let deadline = Instant::now() + retry_policy.timeout;
    let test_dial_addr = test_tor_onion_dial_addr();
    let mut tor_client = None;
    let mut last_error = anyhow!("node probe failed without a retryable error");

    loop {
        if test_dial_addr.is_none() && tor_client.is_none() {
            let config =
                default_client_tor_config().context("build tor node probe client config")?;
            match TorClient::create_bootstrapped(config)
                .await
                .context("bootstrap tor node probe client")
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
                .context("connect node probe")
                .map(|stream| Box::new(stream) as RemoteStream)
                .map_err(ProbeAttemptError::retryable)
        } else {
            tor_client
                .as_ref()
                .expect("tor client should be initialized before connect")
                .connect((host.as_str(), port))
                .await
                .context("connect node probe")
                .map(|stream| Box::new(stream) as RemoteStream)
                .map_err(ProbeAttemptError::retryable)
        };

        match stream {
            Ok(stream) => match probe_once(stream, &authority, bearer_token, base_url).await {
                Ok(node) => return Ok(node),
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

    Err(last_error).context(format!(
        "Tor onion service at {base_url} did not become reachable within {}ms; a freshly started takd hidden service may still be propagating",
        retry_policy.timeout.as_millis()
    ))
}

trait RemoteIo: AsyncRead + tokio::io::AsyncWrite {}
impl<T> RemoteIo for T where T: AsyncRead + tokio::io::AsyncWrite + ?Sized {}
type RemoteStream = Box<dyn RemoteIo + Unpin + Send>;

async fn connect(endpoint: &str, kind: RemoteTransportKind) -> Result<RemoteStream> {
    let (host, port) = endpoint_host_port(endpoint)?;
    if kind == RemoteTransportKind::Direct || !host.ends_with(".onion") {
        return Ok(Box::new(
            TcpStream::connect(socket_addr_from_host_port(&host, port)).await?,
        ));
    }
    if let Some(test_dial_addr) = test_tor_onion_dial_addr() {
        return Ok(Box::new(TcpStream::connect(test_dial_addr).await?));
    }
    let client = TorClient::create_bootstrapped(default_client_tor_config()?).await?;
    Ok(Box::new(client.connect((host.as_str(), port)).await?))
}

async fn probe_once(
    stream: RemoteStream,
    authority: &str,
    bearer_token: &str,
    base_url: &str,
) -> std::result::Result<NodeInfo, ProbeAttemptError> {
    let (status, body) = send_node_info_request(stream, authority, bearer_token, base_url).await?;
    if status != 200 {
        return Err(ProbeAttemptError::final_error(anyhow!(
            "node probe failed with HTTP {status}"
        )));
    }
    NodeInfo::decode(body.as_slice())
        .context("decode node info protobuf")
        .map_err(ProbeAttemptError::final_error)
}

fn endpoint_socket_addr(endpoint: &str) -> Result<String> {
    shared_endpoint_socket_addr(endpoint)
}

fn endpoint_host_port(endpoint: &str) -> Result<(String, u16)> {
    shared_endpoint_host_port(endpoint)
}

async fn send_node_info_request<S>(
    stream: S,
    authority: &str,
    bearer_token: &str,
    base_url: &str,
) -> std::result::Result<(u16, Vec<u8>), ProbeAttemptError>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (mut sender, connection) =
        hyper::client::conn::http1::handshake(hyper_util::rt::TokioIo::new(stream))
            .await
            .with_context(|| format!("malformed HTTP response from {base_url}"))
            .map_err(ProbeAttemptError::retryable)?;
    let _connection_task = AbortOnDrop::new(tokio::spawn(async move {
        let _ = connection.await;
    }));
    let mut request = Request::builder()
        .method("GET")
        .uri("/v1/node/info")
        .header(hyper::header::HOST, authority)
        .header(hyper::header::CONNECTION, "close");
    if !bearer_token.trim().is_empty() {
        request = request.header(
            hyper::header::AUTHORIZATION,
            format!("Bearer {}", bearer_token.trim()),
        );
    }
    let request = request
        .body(Empty::<Bytes>::new())
        .context("write node probe")
        .map_err(ProbeAttemptError::retryable)?;
    let response = sender
        .send_request(request)
        .await
        .with_context(|| format!("malformed HTTP response from {base_url}"))
        .map_err(ProbeAttemptError::retryable)?;
    let status = response.status().as_u16();
    let body = response
        .into_body()
        .collect()
        .await
        .with_context(|| format!("truncated HTTP response body from {base_url}"))
        .map_err(ProbeAttemptError::retryable)?
        .to_bytes()
        .to_vec();
    Ok((status, body))
}

mod remote_probe_connection_cleanup_tests;
#[cfg(test)]
mod remote_probe_tests;
