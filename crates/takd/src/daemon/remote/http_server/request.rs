use super::*;

pub(super) struct ParsedHttpRequest {
    pub(super) method: String,
    pub(super) path: String,
    pub(super) authorization: Option<String>,
    pub(super) body: Option<Vec<u8>>,
}

pub(super) async fn read_http_request<S>(stream: &mut S) -> Result<Option<ParsedHttpRequest>>
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
    let content_length = parse_content_length(&header_text)?;
    let authorization = parse_authorization(&header_text);

    let mut body = request_bytes[header_end..].to_vec();
    while body.len() < content_length {
        let read = stream.read(&mut chunk).await.context("read request body")?;
        if read == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..read]);
    }
    body.truncate(content_length);
    Ok(Some(ParsedHttpRequest {
        method,
        path,
        authorization,
        body: if body.is_empty() { None } else { Some(body) },
    }))
}

pub(super) fn request_parse_error_reason(err: &anyhow::Error) -> &'static str {
    if format!("{err:#}").contains("invalid_content_length") {
        "invalid_content_length"
    } else {
        "invalid_http_request"
    }
}

pub(super) fn request_is_authorized(
    request: &ParsedHttpRequest,
    context: &RemoteNodeContext,
) -> bool {
    if context.bearer_token.trim().is_empty() {
        return true;
    }
    request.authorization.as_deref() == Some(&format!("Bearer {}", context.bearer_token))
}

fn parse_content_length(header_text: &str) -> Result<usize> {
    for line in header_text.lines() {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if !name.trim().eq_ignore_ascii_case("content-length") {
            continue;
        }
        return value
            .trim()
            .parse::<usize>()
            .map_err(|_| anyhow!("invalid_content_length"));
    }
    Ok(0)
}

fn parse_authorization(header_text: &str) -> Option<String> {
    header_text.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name.trim().eq_ignore_ascii_case("authorization") {
            Some(value.trim().to_string())
        } else {
            None
        }
    })
}
