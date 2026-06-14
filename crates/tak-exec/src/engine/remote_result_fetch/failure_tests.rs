#![cfg(test)]
use prost::Message;
use tak_proto::ErrorResponse;

use super::decode_error_detail;

#[test]
fn decode_error_detail_prefers_protobuf_message() {
    let body = ErrorResponse {
        message: "request_failed: database is locked".to_string(),
    }
    .encode_to_vec();
    assert_eq!(
        decode_error_detail(Some(&body)),
        "request_failed: database is locked"
    );
}

#[test]
fn decode_error_detail_falls_back_for_non_protobuf() {
    assert_eq!(decode_error_detail(None), "<no body>");
    // A plainly invalid protobuf byte sequence yields the preview fallback.
    let detail = decode_error_detail(Some(&[0xff, 0xff, 0xff, 0xff]));
    assert!(detail.starts_with("<4 bytes;"), "got: {detail}");
}
