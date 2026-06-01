use super::*;
use http_body_util::Full;
use sha2::{Digest, Sha256};
use tak_proto::{AppendWorkspaceUploadResponse, BeginWorkspaceUploadResponse};

pub(in crate::daemon::remote) async fn drive_h2_workspace_stream<S>(
    client_io: S,
    upload_id: &str,
    body: Vec<u8>,
) -> Result<(AppendWorkspaceUploadResponse, BeginWorkspaceUploadResponse), String>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (mut sender, connection) =
        hyper::client::conn::http2::handshake(TokioExecutor::new(), TokioIo::new(client_io))
            .await
            .map_err(|err| format!("handshake: {err}"))?;
    let conn = tokio::spawn(connection);
    let digest = format!("{:x}", Sha256::digest(&body));
    let request = Request::builder()
        .method("POST")
        .uri(format!(
            "/v2/workspaces/uploads/{upload_id}/stream?offset=0"
        ))
        .header(hyper::header::HOST, "builder-a.onion")
        .header(hyper::header::AUTHORIZATION, "Bearer secret")
        .header("X-Tak-Upload-Sha256", digest)
        .header("X-Tak-Upload-Size", body.len().to_string())
        .body(Full::new(Bytes::from(body)))
        .map_err(|err| format!("build stream request: {err}"))?;
    let response = sender
        .send_request(request)
        .await
        .map_err(|err| format!("send stream request: {err}"))?;
    let uploaded = decode_response::<AppendWorkspaceUploadResponse>(response).await?;
    let request = Request::builder()
        .method("GET")
        .uri(format!("/v2/workspaces/uploads/{upload_id}"))
        .header(hyper::header::HOST, "builder-a.onion")
        .header(hyper::header::AUTHORIZATION, "Bearer secret")
        .body(Full::new(Bytes::new()))
        .map_err(|err| format!("build status request: {err}"))?;
    let response = sender
        .send_request(request)
        .await
        .map_err(|err| format!("send status request: {err}"))?;
    let status = decode_response::<BeginWorkspaceUploadResponse>(response).await?;
    conn.abort();
    Ok((uploaded, status))
}

async fn decode_response<T>(response: hyper::Response<hyper::body::Incoming>) -> Result<T, String>
where
    T: Message + Default,
{
    if response.status() != 200 {
        return Err(format!("unexpected status {}", response.status()));
    }
    let body = response
        .into_body()
        .collect()
        .await
        .map_err(|err| format!("collect body: {err}"))?
        .to_bytes();
    T::decode(body).map_err(|err| format!("decode: {err}"))
}
