use std::sync::{Arc, Mutex};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use super::super::heartbeat::{ping_peer, unix_epoch_ms};
use super::super::{PeerManager, PeerState};
use super::support::{encoded_ping_body, inventory, record};
use crate::daemon::protocol::TorBroker;

// After the first heartbeat learns a peer only answers HTTP/1.1 (its HTTP/2
// attempt fails and the HTTP/1.1 fallback succeeds), later heartbeats must go
// straight to HTTP/1.1 instead of re-dialing a doomed HTTP/2 connection every
// cycle — that doomed redial is what exhausted the heartbeat budget over Tor.
#[tokio::test(flavor = "multi_thread")]
async fn heartbeat_remembers_http1_only_peer_and_skips_http2_next_time() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind ping");
    let addr = listener.local_addr().expect("listener addr");
    let observed: Arc<Mutex<Vec<&'static str>>> = Arc::new(Mutex::new(Vec::new()));
    let server_observed = Arc::clone(&observed);
    // First ping: one closed HTTP/2 probe then a served HTTP/1.1 request.
    // Second ping: a single served HTTP/1.1 request and nothing else.
    let server = tokio::spawn(async move {
        for _ in 0..3 {
            let (mut stream, _) = listener.accept().await.expect("accept");
            let mut buffer = [0_u8; 512];
            let _ = stream.read(&mut buffer).await.unwrap_or(0);
            if buffer.starts_with(b"PRI * HTTP/2") {
                server_observed.lock().expect("lock").push("h2");
                continue; // drop the connection so the HTTP/2 attempt fails
            }
            server_observed.lock().expect("lock").push("h1");
            let body = encoded_ping_body();
            stream
                .write_all(http1_ok_head(body.len()).as_bytes())
                .await
                .expect("write head");
            stream.write_all(&body).await.expect("write body");
        }
    });

    let manager =
        PeerManager::from_inventory(inventory(vec![record("builder-a", "tor", true, "secret")]));
    let target = manager
        .heartbeat_targets_due(unix_epoch_ms())
        .pop()
        .unwrap();
    let broker = TorBroker::for_test_dial_addr(addr.to_string());

    ping_peer(&manager, &broker, &target).await;
    assert_eq!(manager.snapshots()[0].state, PeerState::Connected);

    ping_peer(&manager, &broker, &target).await;
    assert_eq!(manager.snapshots()[0].state, PeerState::Connected);

    // The third accept never happens (only two pings, the second HTTP/1.1 only),
    // so abort the still-listening server instead of awaiting it.
    server.abort();
    let observed = observed.lock().expect("lock").clone();
    assert_eq!(
        observed,
        vec!["h2", "h1", "h1"],
        "second heartbeat must skip the HTTP/2 probe and reuse HTTP/1.1"
    );
}

fn http1_ok_head(body_len: usize) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/x-protobuf\r\nContent-Length: {body_len}\r\nConnection: close\r\n\r\n"
    )
}
