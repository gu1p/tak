use std::future::Future;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail};
use bytes::Bytes;
use http_body_util::{BodyExt, Empty};
use hyper::Request;
use prost::Message;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::time::{sleep, timeout};
use tor_rtcompat::Runtime;

trait RemoteIo: AsyncRead + AsyncWrite {}
impl<T> RemoteIo for T where T: AsyncRead + AsyncWrite + ?Sized {}
type RemoteStream = Box<dyn RemoteIo + Unpin + Send>;
const MAX_PROBE_ATTEMPT_TIMEOUT: Duration = Duration::from_secs(15);

struct AbortOnDrop<T> {
    handle: Option<tokio::task::JoinHandle<T>>,
}

impl<T> AbortOnDrop<T> {
    fn new(handle: tokio::task::JoinHandle<T>) -> Self {
        Self {
            handle: Some(handle),
        }
    }
}

impl<T> Drop for AbortOnDrop<T> {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }
}

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
    stream: RemoteStream,
    authority: &str,
    bearer_token: &str,
    base_url: &str,
) -> Result<()> {
    let (status, body) = send_node_info_request(stream, authority, bearer_token, base_url).await?;
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
    tak_core::endpoint::endpoint_socket_addr(endpoint).map_err(Into::into)
}

fn endpoint_host_port(endpoint: &str) -> Result<(String, u16)> {
    tak_core::endpoint::endpoint_host_port(endpoint).map_err(Into::into)
}

async fn send_node_info_request<S>(
    stream: S,
    authority: &str,
    bearer_token: &str,
    base_url: &str,
) -> Result<(u16, Vec<u8>)>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (mut sender, connection) =
        hyper::client::conn::http1::handshake(hyper_util::rt::TokioIo::new(stream))
            .await
            .with_context(|| format!("malformed HTTP response from {base_url}"))?;
    let _connection_task = AbortOnDrop::new(tokio::spawn(async move {
        let _ = connection.await;
    }));
    let request = Request::builder()
        .method("GET")
        .uri("/v1/node/info")
        .header(hyper::header::HOST, authority)
        .header(
            hyper::header::AUTHORIZATION,
            format!("Bearer {}", bearer_token.trim()),
        )
        .header(hyper::header::CONNECTION, "close")
        .body(Empty::<Bytes>::new())
        .context("write startup node probe")?;
    let response = sender
        .send_request(request)
        .await
        .with_context(|| format!("malformed HTTP response from {base_url}"))?;
    let status = response.status().as_u16();
    let body = response
        .into_body()
        .collect()
        .await
        .with_context(|| format!("truncated HTTP response body from {base_url}"))?
        .to_bytes()
        .to_vec();
    Ok((status, body))
}

mod http_connection_cleanup_tests;
#[cfg(test)]
mod http_response_tests;
#[cfg(test)]
mod http_response_truncated_body_tests;
#[cfg(test)]
mod tests;
