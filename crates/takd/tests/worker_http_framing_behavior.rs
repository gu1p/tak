use crate::support::worker_http::{send_raw_request, start_server};

#[tokio::test]
async fn worker_rejects_ambiguous_or_unsupported_http_body_framing() {
    let server = start_server().await;
    for framing in [
        "Content-Length: 0\r\nContent-Length: 10\r\n",
        "Content-Length: 0\r\ncontent-length: 0\r\n",
        "Content-Length: +0\r\n",
        "Transfer-Encoding: chunked\r\n",
        "Content-Length: 0\r\nTransfer-Encoding: chunked\r\n",
    ] {
        let request = format!(
            "GET /v2/worker/identity HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer secret\r\nX-Tak-Protocol-Version: v2\r\n{framing}\r\n"
        );
        let response = send_raw_request(server.addr, request.as_bytes()).await;
        assert!(
            response.head.starts_with("HTTP/1.1 400 Bad Request\r\n"),
            "accepted invalid framing {framing:?}: {}",
            response.head
        );
    }
}
