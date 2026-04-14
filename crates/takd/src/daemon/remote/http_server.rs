use super::*;

mod request;
mod response;

use request::{read_http_request, request_is_authorized, request_parse_error_reason};
use response::write_http_response;

pub async fn run_remote_v1_http_server(
    listener: TcpListener,
    store: SubmitAttemptStore,
    context: RemoteNodeContext,
) -> Result<()> {
    spawn_remote_cleanup_janitor(context.shared_status_state());
    loop {
        let (stream, _) = listener.accept().await.context("accept failed")?;
        let store = store.clone();
        let context = context.clone();
        tokio::spawn(async move {
            if let Err(err) = handle_remote_v1_http_client(stream, store, context).await {
                tracing::error!("remote v1 http client handling error: {err}");
            }
        });
    }
}

async fn handle_remote_v1_http_client(
    mut stream: TcpStream,
    store: SubmitAttemptStore,
    context: RemoteNodeContext,
) -> Result<()> {
    handle_remote_v1_http_stream(&mut stream, &store, &context).await
}

pub(crate) async fn handle_remote_v1_http_stream<S>(
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
        Err(err) => {
            let response = error_response(400, request_parse_error_reason(&err));
            write_http_response(stream, &response).await?;
            return Ok(());
        }
    };
    if !request_is_authorized(&request, context) {
        write_http_response(stream, &error_response(401, "auth_failed")).await?;
        return Ok(());
    }
    let response = handle_remote_v1_request(
        context,
        store,
        &request.method,
        &request.path,
        request.body.as_deref(),
    )?;
    write_http_response(stream, &response).await?;
    Ok(())
}
