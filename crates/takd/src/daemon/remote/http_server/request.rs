use super::*;

const MAX_REQUEST_HEADER_BYTES: usize = 64 * 1024;

pub(super) struct ParsedHttpRequest {
    pub(super) method: String,
    pub(super) path: String,
    pub(super) headers: Vec<(String, String)>,
    pub(super) body: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RequestParseError {
    InvalidContentLength,
    UnsupportedTransferEncoding,
    HeadersTooLarge,
    IncompleteHeaders,
    TruncatedBody,
    InvalidRequestLine,
}

impl RequestParseError {
    pub(super) fn reason(self) -> &'static str {
        match self {
            Self::InvalidContentLength => "invalid_content_length",
            Self::UnsupportedTransferEncoding => "unsupported_transfer_encoding",
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
    Rejected { status: u16, reason: &'static str },
}

pub(super) async fn read_http_request<S>(
    stream: &mut S,
    context: &RemoteNodeContext,
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
    let headers = parse_headers(&header_text);
    if !authorization_is_valid(header_value(&headers, "authorization"), context) {
        return Err(ReadHttpRequestError::Rejected {
            status: 401,
            reason: "auth_failed",
        });
    }
    let content_length = parse_content_length(&headers).map_err(ReadHttpRequestError::Parse)?;
    if content_length > MAX_REQUEST_BODY_BYTES {
        return Err(ReadHttpRequestError::Rejected {
            status: 413,
            reason: "request_body_too_large",
        });
    }

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
        headers,
        body: if body.is_empty() { None } else { Some(body) },
    }))
}

pub(super) fn authorization_is_valid(
    authorization: Option<&str>,
    context: &RemoteNodeContext,
) -> bool {
    if context.bearer_token.trim().is_empty() {
        return false;
    }
    authorization == Some(format!("Bearer {}", context.bearer_token).as_str())
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

fn parse_content_length(
    headers: &[(String, String)],
) -> std::result::Result<usize, RequestParseError> {
    if header_value(headers, "transfer-encoding").is_some() {
        return Err(RequestParseError::UnsupportedTransferEncoding);
    }
    let mut lengths = headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .map(|(_, value)| value);
    let Some(value) = lengths.next() else {
        return Ok(0);
    };
    if lengths.next().is_some() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(RequestParseError::InvalidContentLength);
    }
    value
        .parse::<usize>()
        .map_err(|_| RequestParseError::InvalidContentLength)
}

fn parse_headers(header_text: &str) -> Vec<(String, String)> {
    header_text
        .lines()
        .skip(1)
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            Some((name.trim().to_string(), value.trim().to_string()))
        })
        .collect()
}

fn header_value<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers.iter().find_map(|(candidate, value)| {
        candidate
            .eq_ignore_ascii_case(name)
            .then_some(value.as_str())
    })
}
