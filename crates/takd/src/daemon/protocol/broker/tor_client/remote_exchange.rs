use super::*;

pub(super) async fn remote_http_exchange(
    broker: &TorBroker,
    remote_request: BrokerRemoteHttpRequest<'_>,
) -> Result<BrokerForwardResponse> {
    let request_headers = remote_headers(
        remote_request.node_id,
        remote_request.bearer_token,
        remote_request.headers,
    );
    let http2_request = BrokerHttp2Request::from_parts(
        remote_request.method,
        remote_request.path,
        request_headers.clone(),
        remote_request.body,
        remote_request.endpoint,
        remote_request.node_id,
    )
    .map_err(anyhow::Error::from)?;
    match broker
        .http2_exchange(remote_request.endpoint, http2_request)
        .await
    {
        Ok(response) => return Ok(response.into_forward_response()),
        Err(err) if !can_fallback_method(remote_request.method, err.code()) => {
            return Err(anyhow::Error::from(err));
        }
        Err(_) => {}
    }
    legacy_http_exchange(
        broker,
        remote_request.endpoint,
        remote_request.method,
        remote_request.path,
        &request_headers,
        remote_request.body,
    )
    .await
    .map_err(anyhow::Error::from)
}

pub(super) fn authorization_value(bearer_token: &str) -> Option<String> {
    let token = bearer_token.trim();
    (!token.is_empty()).then(|| format!("Bearer {token}"))
}

fn remote_headers(
    node_id: &str,
    bearer_token: &str,
    headers: &[(String, String)],
) -> Vec<(String, String)> {
    let mut request_headers = vec![
        ("X-Tak-Protocol-Version".to_string(), "v1".to_string()),
        (REMOTE_NODE_HEADER.to_string(), node_id.to_string()),
    ];
    if let Some(auth) = authorization_value(bearer_token) {
        request_headers.push((hyper::header::AUTHORIZATION.as_str().to_string(), auth));
    }
    request_headers.extend(headers.iter().cloned());
    request_headers
}

fn can_fallback_method(method: &str, code: &str) -> bool {
    code == "http2_unavailable"
        || matches!(method, "GET" | "HEAD" | "OPTIONS" | "PUT" | "DELETE")
            && matches!(code, "connect_failed" | "http2_request_failed")
}

async fn legacy_http_exchange(
    broker: &TorBroker,
    endpoint: &str,
    method: &str,
    path: &str,
    headers: &[(String, String)],
    body: &[u8],
) -> std::result::Result<BrokerForwardResponse, BrokerHttpError> {
    let mut remote = broker
        .connect(endpoint)
        .await
        .map_err(|err| BrokerHttpError::bad_gateway("connect_failed", err))?;
    let payload = legacy_http_request_bytes(endpoint, method, path, headers, body)
        .map_err(|err| BrokerHttpError::bad_request_with_source("invalid_request", err))?;
    remote
        .write_all(&payload)
        .await
        .map_err(|err| BrokerHttpError::bad_gateway("write_failed", err))?;
    let response = super::super::legacy_http::read_remote_http_response(&mut remote).await?;
    parse_forward_response(response).map_err(|err| BrokerHttpError::bad_gateway("read_failed", err))
}

fn legacy_http_request_bytes(
    endpoint: &str,
    method: &str,
    path: &str,
    headers: &[(String, String)],
    body: &[u8],
) -> Result<Vec<u8>> {
    let authority = tak_core::endpoint::endpoint_socket_addr(endpoint)?;
    let mut bytes =
        format!("{method} {path} HTTP/1.1\r\nHost: {authority}\r\nConnection: close\r\n")
            .into_bytes();
    for (name, value) in headers.iter().filter(|(name, _)| keep_forward_header(name)) {
        bytes.extend_from_slice(name.as_bytes());
        bytes.extend_from_slice(b": ");
        bytes.extend_from_slice(value.as_bytes());
        bytes.extend_from_slice(b"\r\n");
    }
    if !body.is_empty() {
        bytes.extend_from_slice(format!("Content-Length: {}\r\n", body.len()).as_bytes());
    }
    bytes.extend_from_slice(b"\r\n");
    bytes.extend_from_slice(body);
    Ok(bytes)
}

fn keep_forward_header(name: &str) -> bool {
    !name.eq_ignore_ascii_case("host")
        && !name.eq_ignore_ascii_case("connection")
        && !name.eq_ignore_ascii_case("content-length")
        && !name.eq_ignore_ascii_case(BROKER_VERSION_HEADER)
        && !name.eq_ignore_ascii_case(REMOTE_ENDPOINT_HEADER)
        && !name.eq_ignore_ascii_case(REMOTE_PROTOCOL_HEADER)
        && !name.eq_ignore_ascii_case(REMOTE_TRANSPORT_HEADER)
}

fn parse_forward_response(response: Vec<u8>) -> Result<BrokerForwardResponse> {
    let header_end = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| index + 4)
        .ok_or_else(|| anyhow::anyhow!("remote response missing HTTP header terminator"))?;
    let head = String::from_utf8_lossy(&response[..header_end]);
    let status = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or_else(|| anyhow::anyhow!("remote response missing HTTP status"))?;
    let headers = head
        .lines()
        .skip(1)
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            Some((name.trim().to_string(), value.trim().to_string()))
        })
        .collect();
    Ok(BrokerForwardResponse {
        status,
        headers,
        body: response[header_end..].to_vec(),
    })
}
