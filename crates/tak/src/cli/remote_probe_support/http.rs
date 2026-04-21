use anyhow::{Context, Result};
use bytes::Bytes;
use http_body_util::{BodyExt, Empty};
use hyper::Request;
use tokio::io::{AsyncRead, AsyncWrite};

use super::{AbortOnDrop, ProbeAttemptError};

pub(in crate::cli) async fn send_http_get<S>(
    stream: S,
    authority: &str,
    path: &str,
    bearer_token: &str,
    base_url: &str,
    write_context: &'static str,
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
    let request = build_get_request(path, authority, bearer_token)
        .context(write_context)
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

pub(super) fn build_get_request(
    path: &str,
    authority: &str,
    bearer_token: &str,
) -> Result<Request<Empty<Bytes>>> {
    let mut request = Request::builder()
        .method("GET")
        .uri(path)
        .header(hyper::header::HOST, authority)
        .header(hyper::header::CONNECTION, "close");
    if let Some(trimmed_token) = trimmed_bearer_token(bearer_token) {
        request = request.header(
            hyper::header::AUTHORIZATION,
            format!("Bearer {trimmed_token}"),
        );
    }
    request.body(Empty::<Bytes>::new()).map_err(Into::into)
}

fn trimmed_bearer_token(bearer_token: &str) -> Option<&str> {
    let trimmed = bearer_token.trim();
    (!trimmed.is_empty()).then_some(trimmed)
}
