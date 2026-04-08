mod support;

use std::io::Write;
use std::net::TcpListener;
use std::process::Command as StdCommand;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::thread;

use prost::Message;
use support::remote_cli::{node_info, read_request, remote_token};

#[test]
fn remote_add_retries_retryable_tor_probe_failures_before_succeeding() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind node info server");
    let port = listener.local_addr().expect("listener addr").port();
    let attempts = Arc::new(AtomicUsize::new(0));
    let server_attempts = Arc::clone(&attempts);

    let server = thread::spawn(move || {
        for attempt in 1..=2 {
            let (mut stream, _) = listener.accept().expect("accept probe");
            assert!(read_request(&mut stream).starts_with("GET /v1/node/info HTTP/1.1\r\n"));
            server_attempts.fetch_add(1, Ordering::SeqCst);
            if attempt == 1 {
                continue;
            }
            let body = node_info(
                "builder-retry",
                "http://builder-retry-hidden-service.onion",
                "tor",
            )
            .encode_to_vec();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/x-protobuf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .expect("write response");
            stream.write_all(&body).expect("write protobuf body");
        }
    });

    let add = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"))
        .args([
            "remote",
            "add",
            &remote_token(
                "builder-retry",
                "http://builder-retry-hidden-service.onion",
                "tor",
            ),
        ])
        .env("XDG_CONFIG_HOME", &config_root)
        .env("TAK_TEST_TOR_ONION_DIAL_ADDR", format!("127.0.0.1:{port}"))
        .env("TAK_TEST_TOR_PROBE_TIMEOUT_MS", "200")
        .env("TAK_TEST_TOR_PROBE_BACKOFF_MS", "10")
        .output()
        .expect("run tak remote add");

    assert!(
        add.status.success(),
        "tak remote add should succeed after retry"
    );
    assert_eq!(attempts.load(Ordering::SeqCst), 2, "expected one retry");
    server.join().expect("probe server should exit");
}
