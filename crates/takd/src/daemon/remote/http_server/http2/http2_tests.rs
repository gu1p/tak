use super::{
    MAX_HANDLER_DETAIL_BYTES, MAX_REQUEST_BODY_BYTES, declared_length_exceeds_cap,
    sanitize_handler_detail,
};
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

// The handler-error detail echoed into a 500 body keeps only the first line of
// the error chain and collapses interior whitespace, so multi-line internal
// errors do not bloat or inject into the response.
#[test]
fn sanitize_keeps_first_line_and_collapses_whitespace() {
    let err = anyhow::anyhow!("boom: lock   held\nstack frame 1\nstack frame 2");
    assert_eq!(sanitize_handler_detail(&err), "boom: lock held");
}

#[test]
fn sanitize_strips_control_characters() {
    let err = anyhow::anyhow!("bad\u{0007}value\ttab");
    assert_eq!(sanitize_handler_detail(&err), "bad value tab");
}

#[test]
fn sanitize_truncates_on_char_boundary_with_ellipsis() {
    let long = "x".repeat(MAX_HANDLER_DETAIL_BYTES + 50);
    let detail = sanitize_handler_detail(&anyhow::anyhow!("{long}"));
    assert!(detail.ends_with('…'));
    // The ellipsis is reserved within the cap, so the total never exceeds it.
    assert!(detail.len() <= MAX_HANDLER_DETAIL_BYTES);
    assert_eq!(
        detail.chars().filter(|c| *c == 'x').count(),
        MAX_HANDLER_DETAIL_BYTES - "…".len()
    );
}
