use super::*;

use bytes::Bytes;
use http_body_util::{BodyExt, Full, combinators::BoxBody};

#[path = "http2/response.rs"]
mod response;
pub(super) use response::BrokerHttp2Response;

#[derive(Clone)]
pub(super) struct BrokerHttp2Request {
    method: String,
    path: String,
    authority: String,
    session_node_id: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl BrokerHttp2Request {
    pub(super) fn new(
        request: &LocalBrokerRequest,
        endpoint: &str,
    ) -> std::result::Result<Self, BrokerHttpError> {
        let session_node_id = request.header(REMOTE_NODE_HEADER).unwrap_or_default();
        Self::from_parts(
            request.method(),
            request.path(),
            request.headers().to_vec(),
            request.body(),
            endpoint,
            session_node_id,
        )
    }

    pub(super) fn from_parts(
        method: &str,
        path: &str,
        headers: Vec<(String, String)>,
        body: &[u8],
        endpoint: &str,
        session_node_id: &str,
    ) -> std::result::Result<Self, BrokerHttpError> {
        let authority = tak_core::endpoint::endpoint_socket_addr(endpoint)
            .map_err(|err| BrokerHttpError::bad_request_with_source("invalid_request", err))?;
        Ok(Self {
            method: method.to_string(),
            path: path.to_string(),
            authority,
            session_node_id: session_node_id.to_string(),
            headers,
            body: body.to_vec(),
        })
    }

    pub(super) fn to_hyper_request(
        &self,
    ) -> std::result::Result<hyper::Request<BrokerBody>, BrokerHttpError> {
        let mut request = hyper::Request::builder()
            .method(self.method.as_str())
            .uri(self.path.as_str())
            .header(hyper::header::HOST, self.authority.as_str());
        for (name, value) in self
            .headers
            .iter()
            .filter(|(name, _)| keep_http2_header(name))
        {
            request = request.header(name, value);
        }
        request
            .body(
                Full::new(Bytes::copy_from_slice(&self.body))
                    .map_err(|err| match err {})
                    .boxed(),
            )
            .map_err(|err| BrokerHttpError::bad_request_with_source("invalid_request", err))
    }

    pub(super) fn session_key(&self, endpoint: &str) -> String {
        format!(
            "{}\n{}\n{}",
            endpoint,
            self.session_node_id,
            self.header(hyper::header::AUTHORIZATION.as_str())
                .unwrap_or_default()
        )
    }

    pub(super) fn can_retry_after_failure(&self) -> bool {
        matches!(
            self.method.as_str(),
            "GET" | "HEAD" | "OPTIONS" | "PUT" | "DELETE"
        )
    }

    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(header_name, _)| header_name.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

pub(super) struct BrokerHttp2StreamRequest {
    method: String,
    path: String,
    authority: String,
    session_node_id: String,
    headers: Vec<(String, String)>,
    body: BoxBody<Bytes, std::io::Error>,
}

impl BrokerHttp2StreamRequest {
    pub(super) fn from_parts(
        method: &str,
        path: &str,
        headers: Vec<(String, String)>,
        body: BoxBody<Bytes, std::io::Error>,
        endpoint: &str,
        session_node_id: &str,
    ) -> std::result::Result<Self, BrokerHttpError> {
        let authority = tak_core::endpoint::endpoint_socket_addr(endpoint)
            .map_err(|err| BrokerHttpError::bad_request_with_source("invalid_request", err))?;
        Ok(Self {
            method: method.to_string(),
            path: path.to_string(),
            authority,
            session_node_id: session_node_id.to_string(),
            headers,
            body,
        })
    }

    pub(super) fn into_hyper_request(
        self,
    ) -> std::result::Result<hyper::Request<BrokerBody>, BrokerHttpError> {
        let mut request = hyper::Request::builder()
            .method(self.method.as_str())
            .uri(self.path.as_str())
            .header(hyper::header::HOST, self.authority.as_str());
        for (name, value) in self
            .headers
            .iter()
            .filter(|(name, _)| keep_http2_header(name))
        {
            request = request.header(name, value);
        }
        request
            .body(self.body)
            .map_err(|err| BrokerHttpError::bad_request_with_source("invalid_request", err))
    }

    pub(super) fn session_key(&self, endpoint: &str) -> String {
        format!(
            "{}\n{}\n{}",
            endpoint,
            self.session_node_id,
            self.header(hyper::header::AUTHORIZATION.as_str())
                .unwrap_or_default()
        )
    }

    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(header_name, _)| header_name.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

fn keep_http2_header(name: &str) -> bool {
    !name.eq_ignore_ascii_case("host")
        && !name.eq_ignore_ascii_case("connection")
        && !name.eq_ignore_ascii_case("content-length")
        && !name.eq_ignore_ascii_case(BROKER_VERSION_HEADER)
        && !name.eq_ignore_ascii_case(REMOTE_NODE_HEADER)
        && !name.eq_ignore_ascii_case(REMOTE_ENDPOINT_HEADER)
        && !name.eq_ignore_ascii_case(REMOTE_PROTOCOL_HEADER)
        && !name.eq_ignore_ascii_case(REMOTE_TRANSPORT_HEADER)
}
