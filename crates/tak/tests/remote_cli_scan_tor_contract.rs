#![cfg(target_os = "linux")]

use crate::support;

use std::io::Write;
use std::net::TcpListener;
use std::thread;

use prost::Message;
use support::remote_cli::{node_info, read_request};
use support::remote_scan::{run_scan_with_env, write_single_camera_qr_fixture};
use tak_proto::encode_tor_invite;

#[test]
fn remote_scan_accepts_tor_invite_qr_and_persists_remote_without_bearer_token() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let fixture_path = temp.path().join("scan.toml");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind node info server");
    let port = listener.local_addr().expect("listener addr").port();
    let base_url = "http://builder-scan.onion";
    let invite = encode_tor_invite(base_url).expect("encode tor invite");
    write_single_camera_qr_fixture(&fixture_path, &invite).expect("write scan fixture");

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
        let body = node_info("builder-scan", base_url, "tor").encode_to_vec();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/x-protobuf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .expect("write response");
        stream.write_all(&body).expect("write protobuf body");
    });

    let output = run_scan_with_env(
        &config_root,
        &fixture_path,
        "enter,tick,tick,enter",
        &[("TAK_TEST_TOR_ONION_DIAL_ADDR", format!("127.0.0.1:{port}"))],
    )
    .expect("run scan");

    assert!(
        output.status.success(),
        "tak remote scan should succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Confirm Remote"),
        "missing confirmation UI:\n{stdout}"
    );
    assert!(
        stdout.contains("Base URL: http://builder-scan.onion"),
        "missing invite base url:\n{stdout}"
    );
    assert!(
        stdout.contains("Transport: tor"),
        "missing tor transport info:\n{stdout}"
    );
    assert!(
        stdout.contains("added remote builder-scan"),
        "missing success:\n{stdout}"
    );
    let inventory =
        std::fs::read_to_string(config_root.join("tak/remotes.toml")).expect("inventory");
    assert!(
        inventory.contains("builder-scan"),
        "missing persisted remote:\n{inventory}"
    );
    assert!(
        inventory.contains("bearer_token = \"\""),
        "tor invite scan should persist an empty bearer token:\n{inventory}"
    );

    server.join().expect("probe server should exit");
}
