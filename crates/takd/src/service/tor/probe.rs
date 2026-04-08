use std::future::Future;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail};
use prost::Message;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::time::{sleep, timeout};
use tor_rtcompat::Runtime;

mod http;

trait RemoteIo: AsyncRead + AsyncWrite {}
impl<T> RemoteIo for T where T: AsyncRead + AsyncWrite + ?Sized {}
type RemoteStream = Box<dyn RemoteIo + Unpin + Send>;
const MAX_PROBE_ATTEMPT_TIMEOUT: Duration = Duration::from_secs(15);

pub(super) async fn wait_for_tor_hidden_service_startup<R>(
    tor_client: &arti_client::TorClient<R>,
    base_url: &str,
    bearer_token: &str,
    timeout: Duration,
    backoff: Duration,
) -> Result<()>
where
    R: Runtime,
{
    let deadline = Instant::now() + timeout;
    let (host, port) = endpoint_host_port(base_url)?;
    let authority = endpoint_socket_addr(base_url)?;
    let mut last_error = anyhow!("hidden service startup probe failed before a response");

    loop {
        match run_with_attempt_timeout(
            deadline,
            MAX_PROBE_ATTEMPT_TIMEOUT,
            "connect takd hidden-service startup probe",
            tor_client.connect((host.as_str(), port)),
        )
        .await
        .context("connect takd hidden-service startup probe")
        {
            Ok(stream) => {
                match run_with_attempt_timeout(
                    deadline,
                    MAX_PROBE_ATTEMPT_TIMEOUT,
                    "probe takd hidden-service startup endpoint",
                    probe_node_info(Box::new(stream), &authority, bearer_token, base_url),
                )
                .await
                {
                    Ok(()) => return Ok(()),
                    Err(err) => last_error = err,
                }
            }
            Err(err) => last_error = err,
        }
        if Instant::now() >= deadline {
            break;
        }
        sleep(backoff).await;
    }

    Err(last_error).context(format!(
        "Tor onion service at {base_url} did not become reachable within {}ms during takd startup",
        timeout.as_millis()
    ))
}

async fn probe_node_info(
    mut stream: RemoteStream,
    authority: &str,
    bearer_token: &str,
    base_url: &str,
) -> Result<()> {
    let request = format!(
        "GET /v1/node/info HTTP/1.1\r\nHost: {authority}\r\nAuthorization: Bearer {bearer_token}\r\nConnection: close\r\n\r\n"
    );
    stream
        .write_all(request.as_bytes())
        .await
        .context("write startup node probe")?;
    stream.flush().await.context("flush startup node probe")?;
    let (status, body) = http::read_http_response(&mut stream, base_url).await?;
    if status != 200 {
        bail!("node probe failed with HTTP {status}");
    }
    tak_proto::NodeInfo::decode(body.as_slice()).context("decode node info protobuf")?;
    Ok(())
}

async fn run_with_attempt_timeout<T, F, E>(
    deadline: Instant,
    max_timeout: Duration,
    stage: &str,
    future: F,
) -> Result<T>
where
    F: Future<Output = std::result::Result<T, E>>,
    E: Into<anyhow::Error>,
{
    let attempt_timeout = deadline
        .saturating_duration_since(Instant::now())
        .min(max_timeout);
    if attempt_timeout.is_zero() {
        bail!("{stage} timed out before the attempt started");
    }
    timeout(attempt_timeout, future)
        .await
        .map_err(|_| anyhow!("{stage} timed out after {}ms", attempt_timeout.as_millis()))?
        .map_err(Into::into)
}

fn endpoint_socket_addr(endpoint: &str) -> Result<String> {
    let trimmed = endpoint.trim();
    let (scheme, without_scheme) = if let Some(value) = trimmed.strip_prefix("http://") {
        ("http", value)
    } else if let Some(value) = trimmed.strip_prefix("https://") {
        ("https", value)
    } else {
        ("", trimmed)
    };
    let authority_end = without_scheme
        .find(['/', '?', '#'])
        .unwrap_or(without_scheme.len());
    let authority = without_scheme[..authority_end].trim();
    if authority.is_empty() {
        bail!("missing host:port");
    }
    if authority.contains(':') {
        return Ok(authority.to_string());
    }
    if scheme.is_empty() {
        bail!("missing port in endpoint authority");
    }
    Ok(format!(
        "{authority}:{}",
        if scheme == "https" { "443" } else { "80" }
    ))
}

fn endpoint_host_port(endpoint: &str) -> Result<(String, u16)> {
    let socket_addr = endpoint_socket_addr(endpoint)?;
    let (host, raw_port) = socket_addr
        .rsplit_once(':')
        .ok_or_else(|| anyhow!("missing host:port"))?;
    if host.trim().is_empty() {
        bail!("missing host");
    }
    Ok((
        host.to_string(),
        raw_port
            .parse::<u16>()
            .with_context(|| format!("invalid port `{raw_port}`"))?,
    ))
}

#[cfg(test)]
mod http_response_tests;
#[cfg(test)]
mod tests;
