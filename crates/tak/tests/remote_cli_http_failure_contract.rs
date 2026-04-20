use crate::support;

use std::io::Write;
use std::net::TcpListener;
use std::process::Command as StdCommand;
use std::thread;

use support::remote_cli::{read_request, remote_inventory_path};
use tak_proto::encode_tor_invite;

#[test]
fn remote_add_does_not_retry_http_failures_and_does_not_persist_remote() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind node info server");
    let port = listener.local_addr().expect("listener addr").port();

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept probe");
        let request = read_request(&mut stream);
        assert!(
            request.starts_with("GET /v1/node/info HTTP/1.1\r\n"),
            "unexpected request: {request}"
        );
        assert!(
            !request.contains("Authorization:"),
            "tor invite probe should not send auth header:\n{request}"
        );
        write!(
            stream,
            "HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        )
        .expect("write response");
    });
    let invite =
        encode_tor_invite("http://builder-auth-hidden-service.onion").expect("encode invite");

    let add = StdCommand::new(support::tak_bin())
        .args(["remote", "add", &invite])
        .env("XDG_CONFIG_HOME", &config_root)
        .env("TAK_TEST_TOR_ONION_DIAL_ADDR", format!("127.0.0.1:{port}"))
        .env("TAK_TEST_TOR_PROBE_TIMEOUT_MS", "200")
        .env("TAK_TEST_TOR_PROBE_BACKOFF_MS", "10")
        .output()
        .expect("run tak remote add");

    let stderr = String::from_utf8_lossy(&add.stderr);
    assert!(!add.status.success(), "tak remote add should fail");
    assert!(
        stderr.contains(
            "failed to probe remote node at http://builder-auth-hidden-service.onion via tor"
        ),
        "unexpected stderr:\n{stderr}"
    );
    assert!(stderr.contains("node probe failed with HTTP 401"));
    assert!(!remote_inventory_path(&config_root).exists());
    server.join().expect("probe server should exit");
}
