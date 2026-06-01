use super::*;

#[path = "request/head.rs"]
mod head;

pub(super) use head::LocalBrokerRequestHead;

const MAX_REQUEST_HEADER_BYTES: usize = 64 * 1024;
const MAX_REQUEST_BODY_BYTES: usize = 512 * 1024 * 1024;

pub(super) struct LocalBrokerRequest {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl LocalBrokerRequest {
    pub(super) fn method(&self) -> &str {
        &self.method
    }

    pub(super) fn path(&self) -> &str {
        &self.path
    }

    pub(super) fn headers(&self) -> &[(String, String)] {
        &self.headers
    }

    pub(super) fn body(&self) -> &[u8] {
        &self.body
    }

    pub(super) fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    pub(super) fn forward_bytes(&self, endpoint: &str) -> Result<Vec<u8>> {
        let authority = tak_core::endpoint::endpoint_socket_addr(endpoint)?;
        let mut bytes = format!(
            "{} {} HTTP/1.1\r\nHost: {authority}\r\nConnection: close\r\n",
            self.method, self.path
        )
        .into_bytes();
        for (name, value) in self.headers.iter().filter(|(name, _)| keep_header(name)) {
            bytes.extend_from_slice(name.as_bytes());
            bytes.extend_from_slice(b": ");
            bytes.extend_from_slice(value.as_bytes());
            bytes.extend_from_slice(b"\r\n");
        }
        if !self.body.is_empty() {
            bytes.extend_from_slice(format!("Content-Length: {}\r\n", self.body.len()).as_bytes());
        }
        bytes.extend_from_slice(b"\r\n");
        bytes.extend_from_slice(&self.body);
        Ok(bytes)
    }
}

pub(super) async fn parse_broker_request<R>(
    first_line: String,
    reader: &mut R,
) -> std::result::Result<LocalBrokerRequest, BrokerHttpError>
where
    R: AsyncBufRead + Unpin,
{
    let (method, path) = parse_request_line(&first_line)?;
    let header_lines = read_header_lines(reader, first_line.len()).await?;
    let headers = parse_headers(&header_lines);
    let content_length = content_length(&headers)?;
    let mut body = vec![0_u8; content_length];
    reader
        .read_exact(&mut body)
        .await
        .map_err(|err| BrokerHttpError::bad_request_with_source("truncated_body", err))?;
    Ok(LocalBrokerRequest {
        method,
        path,
        headers,
        body,
    })
}

pub(super) async fn parse_broker_request_head<R>(
    first_line: String,
    reader: &mut R,
) -> std::result::Result<LocalBrokerRequestHead, BrokerHttpError>
where
    R: AsyncBufRead + Unpin,
{
    let (method, path) = parse_request_line(&first_line)?;
    let header_lines = read_header_lines(reader, first_line.len()).await?;
    let headers = parse_headers(&header_lines);
    let content_length = content_length(&headers)?;
    Ok(LocalBrokerRequestHead::new(
        method,
        path,
        headers,
        content_length,
    ))
}

fn parse_request_line(line: &str) -> std::result::Result<(String, String), BrokerHttpError> {
    let mut parts = line.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| BrokerHttpError::bad_request("invalid_request_line"))?;
    let path = parts
        .next()
        .ok_or_else(|| BrokerHttpError::bad_request("invalid_request_line"))?;
    Ok((method.to_string(), path.to_string()))
}

async fn read_header_lines<R>(
    reader: &mut R,
    mut bytes_read: usize,
) -> std::result::Result<Vec<String>, BrokerHttpError>
where
    R: AsyncBufRead + Unpin,
{
    let mut lines = Vec::new();
    loop {
        let mut line = String::new();
        let read = reader
            .read_line(&mut line)
            .await
            .map_err(|err| BrokerHttpError::bad_request_with_source("read_headers", err))?;
        if read == 0 {
            return Err(BrokerHttpError::bad_request("incomplete_headers"));
        }
        bytes_read += read;
        if bytes_read > MAX_REQUEST_HEADER_BYTES {
            return Err(BrokerHttpError::bad_request("headers_too_large"));
        }
        if line.trim_end().is_empty() {
            return Ok(lines);
        }
        lines.push(line);
    }
}

fn parse_headers(lines: &[String]) -> Vec<(String, String)> {
    lines
        .iter()
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            Some((name.trim().to_string(), value.trim().to_string()))
        })
        .collect()
}

fn content_length(headers: &[(String, String)]) -> std::result::Result<usize, BrokerHttpError> {
    let Some((_, value)) = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
    else {
        return Ok(0);
    };
    let content_length = value
        .parse::<usize>()
        .map_err(|_| BrokerHttpError::bad_request("invalid_content_length"))?;
    if content_length > MAX_REQUEST_BODY_BYTES {
        return Err(BrokerHttpError::bad_request("body_too_large"));
    }
    Ok(content_length)
}

fn keep_header(name: &str) -> bool {
    !name.eq_ignore_ascii_case("host")
        && !name.eq_ignore_ascii_case("connection")
        && !name.eq_ignore_ascii_case("content-length")
        && !name.eq_ignore_ascii_case(BROKER_VERSION_HEADER)
        && !name.eq_ignore_ascii_case(REMOTE_NODE_HEADER)
        && !name.eq_ignore_ascii_case(REMOTE_ENDPOINT_HEADER)
        && !name.eq_ignore_ascii_case(REMOTE_PROTOCOL_HEADER)
        && !name.eq_ignore_ascii_case(REMOTE_TRANSPORT_HEADER)
}
