use crate::support::remote_v1_http::{decode_error_response, send_raw_request, start_server};

const MAX_REQUEST_HEADER_BYTES: usize = 64 * 1024;

#[tokio::test]
async fn missing_header_terminator_returns_explicit_bad_request_reason() {
    let server = start_server().await;
    let response = send_raw_request(
        server.addr,
        concat!(
            "GET /v1/node/info HTTP/1.1\r\n",
            "Host: 127.0.0.1\r\n",
            "Authorization: Bearer secret\r\n",
            "Connection: close\r\n"
        )
        .as_bytes(),
    )
    .await;
    assert!(response.head.starts_with("HTTP/1.1 400 Bad Request\r\n"));
    assert_eq!(
        decode_error_response(&response).message,
        "incomplete_headers"
    );
}

#[tokio::test]
async fn oversized_request_headers_return_explicit_bad_request_reason() {
    let server = start_server().await;
    let oversized_header = "a".repeat(MAX_REQUEST_HEADER_BYTES + 1024);
    let request = format!(
        concat!(
            "GET /v1/node/info HTTP/1.1\r\n",
            "Host: 127.0.0.1\r\n",
            "Authorization: Bearer secret\r\n",
            "X-Fill: {}\r\n",
            "Connection: close\r\n",
            "\r\n"
        ),
        oversized_header
    );
    let response = send_raw_request(server.addr, request.as_bytes()).await;
    assert!(response.head.starts_with("HTTP/1.1 400 Bad Request\r\n"));
    assert_eq!(
        decode_error_response(&response).message,
        "headers_too_large"
    );
}
