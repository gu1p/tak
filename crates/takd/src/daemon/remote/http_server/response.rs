use std::io;

use super::*;

pub(super) async fn write_http_response<S>(
    stream: &mut S,
    response: &RemoteV1Response,
) -> Result<()>
where
    S: AsyncWrite + Unpin,
{
    let status = http_status_line(response.status_code);
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        response.content_type,
        response.body.len(),
    );
    stream
        .write_all(head.as_bytes())
        .await
        .context("write response head")?;
    stream
        .write_all(&response.body)
        .await
        .context("write response bytes")?;
    finalize_http_response(stream).await
}

async fn finalize_http_response<S>(stream: &mut S) -> Result<()>
where
    S: AsyncWrite + Unpin,
{
    if let Err(err) = stream.flush().await {
        if is_disconnect_error(&err) {
            return Ok(());
        }
        return Err(err).context("flush response bytes");
    }
    if let Err(err) = stream.shutdown().await {
        if is_disconnect_error(&err) {
            return Ok(());
        }
        return Err(err).context("shutdown response bytes");
    }
    Ok(())
}

fn is_disconnect_error(err: &io::Error) -> bool {
    matches!(
        err.kind(),
        io::ErrorKind::BrokenPipe
            | io::ErrorKind::ConnectionReset
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::NotConnected
            | io::ErrorKind::UnexpectedEof
    )
}

fn http_status_line(status_code: u16) -> &'static str {
    match status_code {
        200 => "200 OK",
        202 => "202 Accepted",
        400 => "400 Bad Request",
        401 => "401 Unauthorized",
        403 => "403 Forbidden",
        404 => "404 Not Found",
        500 => "500 Internal Server Error",
        _ => "500 Internal Server Error",
    }
}
