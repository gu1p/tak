use anyhow::{Context, Result, bail};
use bytes::Bytes;
use http_body_util::{BodyExt, Empty};
use hyper::Request;
use prost::Message;
use tak_proto::NodeStatusResponse;
use tokio::io::{AsyncRead, AsyncWrite};

pub async fn fetch_node_status<S>(
    stream: S,
    authority: &str,
    bearer_token: &str,
    base_url: &str,
) -> Result<NodeStatusResponse>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (status, body) =
        send_node_status_request(stream, authority, bearer_token, base_url).await?;
    if status != 200 {
        bail!("node probe failed with HTTP {status}");
    }
    let status = NodeStatusResponse::decode(body.as_slice()).context("decode onion node status")?;
    let has_cpu = status
        .cpu
        .as_ref()
        .and_then(|cpu| cpu.tak_admission_available_cores)
        .is_some_and(|cores| cores > 0.0);
    let has_memory = status
        .memory
        .as_ref()
        .and_then(|memory| memory.tak_admission_available_bytes)
        .is_some_and(|bytes| bytes > 0);
    if !has_cpu || !has_memory {
        bail!("node status has not published remote admission capacity");
    }
    Ok(status)
}

async fn send_node_status_request<S>(
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
    let connection_task = tokio::spawn(async move {
        let _ = connection.await;
    });
    let result = async {
        let request = Request::builder()
            .method("GET")
            .uri("/v1/node/status")
            .header(hyper::header::HOST, authority)
            .header(
                hyper::header::AUTHORIZATION,
                format!("Bearer {}", bearer_token.trim()),
            )
            .header(hyper::header::CONNECTION, "close")
            .body(Empty::<Bytes>::new())
            .context("write onion node request")?;
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
    .await;
    connection_task.abort();
    result
}
