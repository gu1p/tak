use http_body_util::BodyExt;

use super::{RemoteV1Response, error_response};

// Hard ceiling on a buffered request body, matching the broker's response cap.
// With the larger receive windows, this bounds how much an authenticated peer
// can make the server allocate for one request.
pub(super) const MAX_REQUEST_BODY_BYTES: usize = 512 * 1024 * 1024;

pub(super) fn declared_length_exceeds_cap(headers: &hyper::HeaderMap) -> bool {
    headers
        .get(hyper::header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<usize>().ok())
        .is_some_and(|length| length > MAX_REQUEST_BODY_BYTES)
}

// Stream the request body, enforcing the size cap as frames arrive so an
// oversized upload is rejected without first buffering all of it.
pub(super) async fn collect_body_capped(
    body: hyper::body::Incoming,
) -> std::result::Result<Vec<u8>, RemoteV1Response> {
    let mut body = body;
    let mut bytes = Vec::new();
    while let Some(frame) = body.frame().await {
        let frame = frame.map_err(|_| error_response(400, "truncated_body"))?;
        if let Some(data) = frame.data_ref() {
            if bytes.len().saturating_add(data.len()) > MAX_REQUEST_BODY_BYTES {
                return Err(error_response(413, "request_body_too_large"));
            }
            bytes.extend_from_slice(data);
        }
    }
    Ok(bytes)
}
