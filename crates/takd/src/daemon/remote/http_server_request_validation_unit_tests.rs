use prost::Message;
use tak_proto::ErrorResponse;

use super::handle_remote_v1_http_stream;
use super::http_server_test_support::{ScriptedHttpStream, node_context, store};

#[tokio::test]
async fn stream_handler_returns_explicit_bad_request_for_incomplete_headers() {
    let request = concat!(
        "GET /v1/node/info HTTP/1.1\r\n",
        "Host: builder-a.onion\r\n",
        "Connection: close\r\n"
    );
    let mut stream = ScriptedHttpStream::with_request(request);
    let (_temp, store) = store();
    let result = handle_remote_v1_http_stream(&mut stream, &store, &node_context()).await;
    assert!(result.is_ok());
    assert_eq!(
        decode_error_response(&stream.written_bytes).message,
        "incomplete_headers"
    );
}

#[tokio::test]
async fn stream_handler_returns_explicit_bad_request_for_oversized_headers() {
    let oversized_header = "a".repeat((64 * 1024) + 1024);
    let request = format!(
        concat!(
            "GET /v1/node/info HTTP/1.1\r\n",
            "Host: builder-a.onion\r\n",
            "X-Fill: {}\r\n",
            "Connection: close\r\n",
            "\r\n"
        ),
        oversized_header
    );
    let mut stream = ScriptedHttpStream::with_request(&request);
    let (_temp, store) = store();
    let result = handle_remote_v1_http_stream(&mut stream, &store, &node_context()).await;
    assert!(result.is_ok());
    assert_eq!(
        decode_error_response(&stream.written_bytes).message,
        "headers_too_large"
    );
}

fn decode_error_response(response: &[u8]) -> ErrorResponse {
    let split = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| index + 4)
        .expect("response should contain header terminator");
    ErrorResponse::decode(&response[split..]).expect("decode error response")
}
