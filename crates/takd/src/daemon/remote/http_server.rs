use super::*;

pub async fn run_remote_v1_http_server(
    listener: TcpListener,
    store: SubmitAttemptStore,
) -> Result<()> {
    loop {
        let (stream, _) = listener.accept().await.context("accept failed")?;
        let store = store.clone();
        tokio::spawn(async move {
            if let Err(err) = handle_remote_v1_http_client(stream, store).await {
                eprintln!("remote v1 http client handling error: {err}");
            }
        });
    }
}

async fn handle_remote_v1_http_client(
    mut stream: TcpStream,
    store: SubmitAttemptStore,
) -> Result<()> {
    handle_remote_v1_http_stream(&mut stream, &store).await
}

pub(super) async fn handle_remote_v1_http_stream<S>(
    stream: &mut S,
    store: &SubmitAttemptStore,
) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let Some(request) = read_http_request(stream).await? else {
        return Ok(());
    };
    let response = handle_remote_v1_request(
        store,
        &request.method,
        &request.path,
        request.body.as_deref(),
    )?;
    write_http_response(stream, &response).await?;
    Ok(())
}

struct ParsedHttpRequest {
    method: String,
    path: String,
    body: Option<String>,
}

async fn read_http_request<S>(stream: &mut S) -> Result<Option<ParsedHttpRequest>>
where
    S: AsyncRead + Unpin,
{
    let mut request_bytes = Vec::new();
    let mut chunk = [0_u8; 1024];
    let mut header_end = None;

    while header_end.is_none() {
        let read = stream
            .read(&mut chunk)
            .await
            .context("read request bytes")?;
        if read == 0 {
            break;
        }
        request_bytes.extend_from_slice(&chunk[..read]);
        header_end = request_bytes
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .map(|idx| idx + 4);
    }

    if request_bytes.is_empty() {
        return Ok(None);
    }

    let header_end = header_end.unwrap_or(request_bytes.len());
    let header_text = String::from_utf8_lossy(&request_bytes[..header_end]);
    let request_line = header_text.lines().next().unwrap_or_default();
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let path = parts.next().unwrap_or("/").to_string();

    let content_length = header_text
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name.trim().eq_ignore_ascii_case("content-length") {
                value.trim().parse::<usize>().ok()
            } else {
                None
            }
        })
        .unwrap_or(0);

    let mut body = request_bytes[header_end..].to_vec();
    while body.len() < content_length {
        let read = stream.read(&mut chunk).await.context("read request body")?;
        if read == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..read]);
    }
    body.truncate(content_length);
    let body = if body.is_empty() {
        None
    } else {
        Some(String::from_utf8_lossy(&body).to_string())
    };

    Ok(Some(ParsedHttpRequest { method, path, body }))
}

async fn write_http_response<S>(stream: &mut S, response: &RemoteV1Response) -> Result<()>
where
    S: AsyncWrite + Unpin,
{
    let status = http_status_line(response.status_code);
    let encoded = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        response.content_type,
        response.body.len(),
        response.body
    );
    stream
        .write_all(encoded.as_bytes())
        .await
        .context("write response bytes")?;
    stream.flush().await.context("flush response bytes")?;
    Ok(())
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
