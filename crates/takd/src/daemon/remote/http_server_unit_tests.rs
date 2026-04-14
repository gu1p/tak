use std::io;

use super::handle_remote_v1_http_stream;
use super::http_server_test_support::{ScriptedHttpStream, node_context, request_bytes, store};

#[tokio::test]
async fn stream_handler_tolerates_disconnect_during_flush_after_full_response_write() {
    let mut stream = ScriptedHttpStream::with_request(request_bytes());
    stream.flush_error = Some(io::ErrorKind::BrokenPipe);
    let (_temp, store) = store();

    let result = handle_remote_v1_http_stream(&mut stream, &store, &node_context()).await;

    assert!(
        result.is_ok(),
        "flush disconnect should be ignored: {result:?}"
    );
    let response = String::from_utf8(stream.written_bytes).expect("utf8 response");
    assert!(
        response.starts_with("HTTP/1.1 200 OK\r\n"),
        "response: {response}"
    );
}

#[tokio::test]
async fn stream_handler_tolerates_disconnect_during_shutdown_after_full_response_write() {
    let mut stream = ScriptedHttpStream::with_request(request_bytes());
    stream.shutdown_error = Some(io::ErrorKind::ConnectionReset);
    let (_temp, store) = store();

    let result = handle_remote_v1_http_stream(&mut stream, &store, &node_context()).await;

    assert!(
        result.is_ok(),
        "shutdown disconnect should be ignored: {result:?}"
    );
    let response = String::from_utf8(stream.written_bytes).expect("utf8 response");
    assert!(
        response.starts_with("HTTP/1.1 200 OK\r\n"),
        "response: {response}"
    );
}

#[tokio::test]
async fn stream_handler_keeps_non_disconnect_flush_failures_fatal() {
    let mut stream = ScriptedHttpStream::with_request(request_bytes());
    stream.flush_error = Some(io::ErrorKind::PermissionDenied);
    let (_temp, store) = store();

    let result = handle_remote_v1_http_stream(&mut stream, &store, &node_context()).await;

    let err = result.expect_err("permission denied should remain fatal");
    assert!(
        format!("{err:#}").contains("flush response bytes"),
        "unexpected error: {err:#}"
    );
}
