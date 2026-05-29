//! A real hyper HTTP/2 client must round-trip against the real
//! `handle_remote_v1_stream` server path, including over transports that behave
//! like arti's Tor `DataStream` (tiny chunked reads/writes, flush-gated writes).
//! These guard the server-side HTTP/2 handling that production heartbeats rely
//! on, which previously had no end-to-end coverage.

use super::handle_remote_v1_stream;
use super::http_server_test_support::{node_context, store};
use super::http2_roundtrip_support::{FlushGatedStream, ThrottledStream, drive_h2_node_info};

#[tokio::test(flavor = "multi_thread")]
async fn http2_round_trip_over_plain_duplex() {
    let (client_io, server_io) = tokio::io::duplex(64 * 1024);
    let (_temp, store) = store();
    let server = tokio::spawn(handle_remote_v1_stream(server_io, store, node_context()));

    let node = drive_h2_node_info(client_io).await.expect("h2 round trip");
    assert_eq!(node.node_id, "builder-a");
    let _ = server.await;
}

#[tokio::test(flavor = "multi_thread")]
async fn http2_round_trip_over_flush_gated_duplex() {
    let (client_io, server_io) = tokio::io::duplex(64 * 1024);
    let client = FlushGatedStream::new(client_io);
    let server = FlushGatedStream::new(server_io);
    let (_temp, store) = store();
    let server_task = tokio::spawn(handle_remote_v1_stream(server, store, node_context()));

    let node = drive_h2_node_info(client)
        .await
        .expect("h2 round trip over flush-gated stream");
    assert_eq!(node.node_id, "builder-a");
    let _ = server_task.await;
}

#[tokio::test(flavor = "multi_thread")]
async fn http2_round_trip_over_byte_chunked_duplex() {
    let (client_io, server_io) = tokio::io::duplex(64 * 1024);
    let client = ThrottledStream::new(client_io, 1, 1);
    let server = ThrottledStream::new(server_io, 1, 1);
    let (_temp, store) = store();
    let server_task = tokio::spawn(handle_remote_v1_stream(server, store, node_context()));

    let node = drive_h2_node_info(client)
        .await
        .expect("h2 round trip over byte-chunked stream");
    assert_eq!(node.node_id, "builder-a");
    let _ = server_task.await;
}
