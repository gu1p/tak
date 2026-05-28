#[path = "remote_output_range_resume_contract/support.rs"]
mod support;

use support::{RangeFixture, register_submit, write_artifact};

#[tokio::test(flavor = "multi_thread")]
async fn remote_output_route_serves_byte_range_for_resume() {
    let temp = tempfile::tempdir().expect("tempdir");
    let fixture = RangeFixture::spawn(&temp).await;
    let key = register_submit(&fixture.store, &fixture.exec_root_base);
    write_artifact(&temp, &key);

    let response = crate::support::remote_v1_http::send_raw_request(
        fixture.addr,
        b"GET /v1/tasks/run-range/outputs?path=out.txt HTTP/1.1\r\nHost: builder\r\nAuthorization: Bearer secret\r\nRange: bytes=6-\r\nConnection: close\r\n\r\n",
    )
    .await;

    assert!(
        response.head.starts_with("HTTP/1.1 206 Partial Content"),
        "unexpected response head: {}",
        response.head
    );
    assert!(response.head.contains("Content-Range: bytes 6-21/22"));
    assert_eq!(response.body, b"resumable output");
}

#[tokio::test(flavor = "multi_thread")]
async fn remote_output_route_serves_bounded_byte_range_without_reading_tail() {
    let temp = tempfile::tempdir().expect("tempdir");
    let fixture = RangeFixture::spawn(&temp).await;
    let key = register_submit(&fixture.store, &fixture.exec_root_base);
    write_artifact(&temp, &key);

    let response = crate::support::remote_v1_http::send_raw_request(
        fixture.addr,
        b"GET /v1/tasks/run-range/outputs?path=out.txt HTTP/1.1\r\nHost: builder\r\nAuthorization: Bearer secret\r\nRange: bytes=6-14\r\nConnection: close\r\n\r\n",
    )
    .await;

    assert!(
        response.head.starts_with("HTTP/1.1 206 Partial Content"),
        "unexpected response head: {}",
        response.head
    );
    assert!(response.head.contains("Content-Range: bytes 6-14/22"));
    assert_eq!(response.body, b"resumable");
}

#[tokio::test(flavor = "multi_thread")]
async fn remote_output_route_rejects_range_start_at_file_length() {
    let temp = tempfile::tempdir().expect("tempdir");
    let fixture = RangeFixture::spawn(&temp).await;
    let key = register_submit(&fixture.store, &fixture.exec_root_base);
    write_artifact(&temp, &key);

    let response = crate::support::remote_v1_http::send_raw_request(
        fixture.addr,
        b"GET /v1/tasks/run-range/outputs?path=out.txt HTTP/1.1\r\nHost: builder\r\nAuthorization: Bearer secret\r\nRange: bytes=22-\r\nConnection: close\r\n\r\n",
    )
    .await;

    assert!(
        response
            .head
            .starts_with("HTTP/1.1 416 Range Not Satisfiable"),
        "unexpected response head: {}",
        response.head
    );
    assert!(response.head.contains("Content-Range: bytes */22"));
    assert!(response.body.is_empty());
}
