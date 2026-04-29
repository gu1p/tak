use anyhow::{Context, Result};
use bytes::Bytes;
use http_body_util::{BodyExt, Empty};
use hyper::Request;
use tokio::io::{AsyncRead, AsyncWrite};

pub(super) trait RemoteIo: AsyncRead + AsyncWrite {}
impl<T> RemoteIo for T where T: AsyncRead + AsyncWrite + ?Sized {}
pub(super) type RemoteStream = Box<dyn RemoteIo + Unpin + Send>;

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

pub(super) async fn send_node_info_request<S>(
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
