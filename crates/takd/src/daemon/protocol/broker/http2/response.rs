use super::*;

use http_body_util::BodyExt;

pub(in crate::daemon::protocol::broker) struct BrokerHttp2Response {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl BrokerHttp2Response {
    pub(in crate::daemon::protocol::broker) async fn from_hyper(
        response: hyper::Response<hyper::body::Incoming>,
    ) -> std::result::Result<Self, BrokerHttpError> {
        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .filter(|(name, _)| keep_response_header(name.as_str()))
            .filter_map(|(name, value)| {
                value
                    .to_str()
                    .ok()
                    .map(|value| (name.as_str().to_string(), value.to_string()))
            })
            .collect();
        let body = collect_body_limited(response).await?;
        Ok(Self {
            status,
            headers,
            body,
        })
    }

    pub(in crate::daemon::protocol::broker) fn to_http1_bytes(
        &self,
    ) -> std::result::Result<Vec<u8>, BrokerHttpError> {
        let mut bytes = format!("HTTP/1.1 {}\r\n", http_status_line(self.status)).into_bytes();
        for (name, value) in &self.headers {
            bytes.extend_from_slice(name.as_bytes());
            bytes.extend_from_slice(b": ");
            bytes.extend_from_slice(value.as_bytes());
            bytes.extend_from_slice(b"\r\n");
        }
        bytes.extend_from_slice(format!("Content-Length: {}\r\n", self.body.len()).as_bytes());
        bytes.extend_from_slice(b"Connection: close\r\n\r\n");
        bytes.extend_from_slice(&self.body);
        Ok(bytes)
    }

    pub(in crate::daemon::protocol::broker) fn into_forward_response(
        self,
    ) -> BrokerForwardResponse {
        BrokerForwardResponse {
            status: self.status,
            headers: self.headers,
            body: self.body,
        }
    }
}

fn keep_response_header(name: &str) -> bool {
    !name.eq_ignore_ascii_case("connection")
        && !name.eq_ignore_ascii_case("transfer-encoding")
        && !name.eq_ignore_ascii_case("content-length")
}

async fn collect_body_limited(
    response: hyper::Response<hyper::body::Incoming>,
) -> std::result::Result<Vec<u8>, BrokerHttpError> {
    ensure_content_length_within_limit(&response)?;
    let mut body = response.into_body();
    let mut bytes = Vec::new();
    while let Some(frame) = body.frame().await {
        let frame = frame.map_err(|err| BrokerHttpError::bad_gateway("http2_body_failed", err))?;
        if let Some(data) = frame.data_ref() {
            if bytes.len().saturating_add(data.len()) > MAX_RESPONSE_BODY_BYTES {
                return Err(BrokerHttpError::bad_gateway(
                    "response_body_too_large",
                    "response body exceeded limit",
                ));
            }
            bytes.extend_from_slice(data);
        }
    }
    Ok(bytes)
}

fn ensure_content_length_within_limit(
    response: &hyper::Response<hyper::body::Incoming>,
) -> std::result::Result<(), BrokerHttpError> {
    let Some(value) = response.headers().get(hyper::header::CONTENT_LENGTH) else {
        return Ok(());
    };
    let Ok(value) = value.to_str() else {
        return Ok(());
    };
    let Ok(length) = value.parse::<usize>() else {
        return Ok(());
    };
    if length > MAX_RESPONSE_BODY_BYTES {
        return Err(BrokerHttpError::bad_gateway(
            "response_body_too_large",
            "response body exceeded limit",
        ));
    }
    Ok(())
}

fn http_status_line(status: u16) -> String {
    match hyper::StatusCode::from_u16(status) {
        Ok(status) => format!(
            "{} {}",
            status.as_u16(),
            status.canonical_reason().unwrap_or("")
        ),
        Err(_) => status.to_string(),
    }
}
