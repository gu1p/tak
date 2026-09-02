use super::*;

mod http2;
mod prefixed_io;
mod request;
mod response;

use http2::handle_worker_http2_stream;
use prefixed_io::{PrefixedIo, read_protocol_prefix};
use request::{ReadHttpRequestError, read_http_request, request_is_authorized};
use response::write_http_response;

pub async fn run_worker_http_server(
    listener: TcpListener,
    store: SubmitAttemptStore,
    context: RemoteNodeContext,
) -> Result<()> {
    spawn_remote_runtime_services(context.clone(), store.clone());
    loop {
        let (stream, _) = listener.accept().await.context("accept failed")?;
        let store = store.clone();
        let context = context.clone();
        tokio::spawn(async move {
            if let Err(err) = handle_worker_stream(stream, store, context).await {
                tracing::error!("worker HTTP client handling error: {err}");
            }
        });
    }
}

pub(crate) async fn handle_worker_stream<S>(
    mut stream: S,
    store: SubmitAttemptStore,
    context: RemoteNodeContext,
) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let prefix = read_protocol_prefix(&mut stream).await?;
    let prefixed = PrefixedIo::new(prefix.bytes, stream);
    if prefix.is_http2 {
        tracing::debug!("serving worker stream over HTTP/2");
        return handle_worker_http2_stream(prefixed, store, context).await;
    }
    tracing::debug!("serving worker stream over HTTP/1.1");
    let mut prefixed = prefixed;
    handle_worker_http_stream(&mut prefixed, &store, &context).await
}

pub(crate) async fn handle_worker_http_stream<S>(
    stream: &mut S,
    store: &SubmitAttemptStore,
    context: &RemoteNodeContext,
) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let request = match read_http_request(stream).await {
        Ok(Some(request)) => request,
        Ok(None) => return Ok(()),
        Err(ReadHttpRequestError::Parse(err)) => {
            let response = error_response(400, err.reason());
            write_http_response(stream, &response).await?;
            return Ok(());
        }
        Err(ReadHttpRequestError::Io(err)) => return Err(err),
    };
    if !request_is_authorized(&request, context) {
        write_http_response(stream, &error_response(401, "auth_failed")).await?;
        return Ok(());
    }
    let response = handle_worker_http_request(
        context,
        store,
        &request.method,
        &request.path,
        &request.headers,
        request.body.as_deref(),
    )?;
    write_http_response(stream, &response).await?;
    Ok(())
}
