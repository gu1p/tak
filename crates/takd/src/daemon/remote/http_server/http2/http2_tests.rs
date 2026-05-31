use super::{MAX_REQUEST_BODY_BYTES, declared_length_exceeds_cap};
use hyper::HeaderMap;
use hyper::header::{CONTENT_LENGTH, HeaderValue};

fn headers_with_content_length(length: u64) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_LENGTH,
        HeaderValue::from_str(&length.to_string()).expect("header value"),
    );
    headers
}

// A request that declares more than the cap is rejected from the header alone,
// before any of the body is buffered.
#[test]
fn declared_length_over_cap_is_rejected() {
    let headers = headers_with_content_length(MAX_REQUEST_BODY_BYTES as u64 + 1);
    assert!(declared_length_exceeds_cap(&headers));
}

#[test]
fn declared_length_within_cap_is_allowed() {
    let headers = headers_with_content_length(MAX_REQUEST_BODY_BYTES as u64);
    assert!(!declared_length_exceeds_cap(&headers));
}

// A missing Content-Length must not be treated as oversized; the streaming
// collector enforces the cap as frames arrive.
#[test]
fn missing_content_length_is_allowed() {
    assert!(!declared_length_exceeds_cap(&HeaderMap::new()));
}
