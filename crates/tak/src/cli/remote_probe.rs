use super::remote_probe_support::{
    ProbeAttemptError, test_tor_onion_dial_addr, tor_probe_retry_policy,
};
use anyhow::{Context, Result, anyhow, bail};
use arti_client::{TorClient, TorClientConfig};
use prost::Message;
use std::time::Instant;
use tak_core::model::RemoteTransportKind;
use tak_exec::{
    endpoint_host_port as shared_endpoint_host_port,
    endpoint_socket_addr as shared_endpoint_socket_addr,
};
use tak_proto::NodeInfo;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
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
            match TorClient::create_bootstrapped(TorClientConfig::default())
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
            TcpStream::connect(format!("{host}:{port}")).await?,
        ));
    }
    if let Some(test_dial_addr) = test_tor_onion_dial_addr() {
        return Ok(Box::new(TcpStream::connect(test_dial_addr).await?));
    }
    let client = TorClient::create_bootstrapped(TorClientConfig::default()).await?;
    Ok(Box::new(client.connect((host.as_str(), port)).await?))
}

async fn probe_once(
    mut stream: RemoteStream,
    authority: &str,
    bearer_token: &str,
    base_url: &str,
) -> std::result::Result<NodeInfo, ProbeAttemptError> {
    let request = format!(
        "GET /v1/node/info HTTP/1.1\r\nHost: {authority}\r\nAuthorization: Bearer {bearer_token}\r\nConnection: close\r\n\r\n"
    );
    stream
        .write_all(request.as_bytes())
        .await
        .context("write node probe")
        .map_err(ProbeAttemptError::retryable)?;
    stream
        .flush()
        .await
        .context("flush node probe")
        .map_err(ProbeAttemptError::retryable)?;

    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .await
        .context("read node probe")
        .map_err(ProbeAttemptError::retryable)?;
    let split = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| index + 4)
        .ok_or_else(|| {
            ProbeAttemptError::retryable(anyhow!("malformed HTTP response from {base_url}"))
        })?;
    let head = String::from_utf8_lossy(&response[..split]);
    let body = &response[split..];
    let status = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or_else(|| {
            ProbeAttemptError::retryable(anyhow!("invalid HTTP status from {base_url}"))
        })?;
    if status != 200 {
        return Err(ProbeAttemptError::final_error(anyhow!(
            "node probe failed with HTTP {status}"
        )));
    }
    NodeInfo::decode(body)
        .context("decode node info protobuf")
        .map_err(ProbeAttemptError::final_error)
}

fn endpoint_socket_addr(endpoint: &str) -> Result<String> {
    shared_endpoint_socket_addr(endpoint)
}

fn endpoint_host_port(endpoint: &str) -> Result<(String, u16)> {
    shared_endpoint_host_port(endpoint)
}
