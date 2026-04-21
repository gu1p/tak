use super::*;

const MAX_REQUEST_HEADER_BYTES: usize = 64 * 1024;

pub(super) struct ParsedHttpRequest {
    pub(super) method: String,
    pub(super) path: String,
    pub(super) authorization: Option<String>,
    pub(super) body: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RequestParseError {
    InvalidContentLength,
    HeadersTooLarge,
    IncompleteHeaders,
    TruncatedBody,
    InvalidRequestLine,
}

impl RequestParseError {
    pub(super) fn reason(self) -> &'static str {
        match self {
            Self::InvalidContentLength => "invalid_content_length",
            Self::HeadersTooLarge => "headers_too_large",
            Self::IncompleteHeaders => "incomplete_headers",
            Self::TruncatedBody => "truncated_body",
            Self::InvalidRequestLine => "invalid_request_line",
        }
    }
}

#[derive(Debug)]
pub(super) enum ReadHttpRequestError {
    Parse(RequestParseError),
    Io(anyhow::Error),
}

pub(super) async fn read_http_request<S>(
    stream: &mut S,
) -> std::result::Result<Option<ParsedHttpRequest>, ReadHttpRequestError>
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
            .context("read request bytes")
            .map_err(ReadHttpRequestError::Io)?;
        if read == 0 {
            break;
        }
        let previous_len = request_bytes.len();
        request_bytes.extend_from_slice(&chunk[..read]);
        let search_start = previous_len.saturating_sub(3);
        if let Some(idx) = request_bytes[search_start..]
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
        {
            let candidate = search_start + idx + 4;
            if candidate > MAX_REQUEST_HEADER_BYTES {
                return Err(ReadHttpRequestError::Parse(
                    RequestParseError::HeadersTooLarge,
                ));
            }
            header_end = Some(candidate);
            break;
        }
        if request_bytes.len() > MAX_REQUEST_HEADER_BYTES {
            return Err(ReadHttpRequestError::Parse(
                RequestParseError::HeadersTooLarge,
            ));
        }
    }

    if request_bytes.is_empty() {
        return Ok(None);
    }

    let header_end = header_end.ok_or(ReadHttpRequestError::Parse(
        RequestParseError::IncompleteHeaders,
    ))?;
    let header_text = String::from_utf8_lossy(&request_bytes[..header_end]);
    let (method, path) = parse_request_line(&header_text).map_err(ReadHttpRequestError::Parse)?;
    let content_length = parse_content_length(&header_text).map_err(ReadHttpRequestError::Parse)?;
    let authorization = parse_authorization(&header_text);

    let mut body = request_bytes[header_end..].to_vec();
    while body.len() < content_length {
        let read = stream
            .read(&mut chunk)
            .await
            .context("read request body")
            .map_err(ReadHttpRequestError::Io)?;
        if read == 0 {
            return Err(ReadHttpRequestError::Parse(
                RequestParseError::TruncatedBody,
            ));
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

pub(super) fn request_is_authorized(
    request: &ParsedHttpRequest,
    context: &RemoteNodeContext,
) -> bool {
    if context
        .node_info()
        .ok()
        .is_some_and(|node| node.transport == "tor")
    {
        return true;
    }
    if context.bearer_token.trim().is_empty() {
        return false;
    }
    request.authorization.as_deref() == Some(&format!("Bearer {}", context.bearer_token))
}

fn parse_request_line(
    header_text: &str,
) -> std::result::Result<(String, String), RequestParseError> {
    let request_line = header_text.lines().next().unwrap_or_default();
    let mut parts = request_line.split_whitespace();
    let Some(method) = parts.next() else {
        return Err(RequestParseError::InvalidRequestLine);
    };
    let Some(path) = parts.next() else {
        return Err(RequestParseError::InvalidRequestLine);
    };
    Ok((method.to_string(), path.to_string()))
}

fn parse_content_length(header_text: &str) -> std::result::Result<usize, RequestParseError> {
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
            .map_err(|_| RequestParseError::InvalidContentLength);
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
