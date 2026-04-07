use anyhow::{Context, Result, anyhow, bail};
use arti_client::{TorClient, TorClientConfig};
use prost::Message;
use tak_core::model::RemoteTransportKind;
use tak_exec::{
    endpoint_host_port as shared_endpoint_host_port,
    endpoint_socket_addr as shared_endpoint_socket_addr,
};
use tak_proto::NodeInfo;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

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
    let authority = endpoint_socket_addr(base_url)?;
    let mut stream = connect(base_url, kind).await?;
    let request = format!(
        "GET /v1/node/info HTTP/1.1\r\nHost: {authority}\r\nAuthorization: Bearer {bearer_token}\r\nConnection: close\r\n\r\n"
    );
    stream
        .write_all(request.as_bytes())
        .await
        .context("write node probe")?;
    stream.flush().await.context("flush node probe")?;

    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .await
        .context("read node probe")?;
    let split = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| index + 4)
        .ok_or_else(|| anyhow!("malformed HTTP response from {base_url}"))?;
    let head = String::from_utf8_lossy(&response[..split]);
    let body = &response[split..];
    let status = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or_else(|| anyhow!("invalid HTTP status from {base_url}"))?;
    if status != 200 {
        bail!("node probe failed with HTTP {status}");
    }
    NodeInfo::decode(body).context("decode node info protobuf")
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
    if let Some(test_dial_addr) = std::env::var("TAK_TEST_TOR_ONION_DIAL_ADDR")
        .ok()
        .filter(|value| !value.trim().is_empty())
    {
        return Ok(Box::new(TcpStream::connect(test_dial_addr).await?));
    }
    let client = TorClient::create_bootstrapped(TorClientConfig::default()).await?;
    Ok(Box::new(client.connect((host.as_str(), port)).await?))
}

fn endpoint_socket_addr(endpoint: &str) -> Result<String> {
    shared_endpoint_socket_addr(endpoint)
}

fn endpoint_host_port(endpoint: &str) -> Result<(String, u16)> {
    shared_endpoint_host_port(endpoint)
}
