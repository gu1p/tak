use super::*;

use http_body_util::BodyExt;

pub(in crate::daemon::protocol::broker) struct BrokerHttp2Response {
    status: u16,
    body: Vec<u8>,
}

impl BrokerHttp2Response {
    pub(in crate::daemon::protocol::broker) async fn from_hyper(
        response: hyper::Response<hyper::body::Incoming>,
    ) -> std::result::Result<Self, BrokerHttpError> {
        let status = response.status().as_u16();
        let body = collect_body_limited(response).await?;
        Ok(Self { status, body })
    }

    pub(in crate::daemon::protocol::broker) fn into_forward_response(
        self,
    ) -> BrokerForwardResponse {
        BrokerForwardResponse {
            status: self.status,
            body: self.body,
        }
    }
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
