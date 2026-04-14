#![cfg(target_os = "linux")]

mod support;

use std::io::Write;
use std::net::TcpListener;
use std::thread;

use prost::Message;
use support::remote_cli::{node_info, read_request, remote_token};
use support::remote_scan::{CameraFixture, FrameFixture, run_scan, write_scan_fixture};

#[test]
fn remote_scan_selects_camera_detects_qr_and_persists_remote_after_confirmation() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let fixture_path = temp.path().join("scan.toml");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind node info server");
    let addr = listener.local_addr().expect("listener addr");
    let base_url = format!("http://{addr}");
    let token = remote_token("builder-scan", &base_url, "direct");
    write_scan_fixture(
        &fixture_path,
        &[CameraFixture {
            id: "cam0",
            name: "Desk Camera",
            frames: &[
                FrameFixture::Blank {
                    width: 192,
                    height: 192,
                },
                FrameFixture::QrPayload {
                    payload: &token,
                    width: 192,
                },
            ],
        }],
    )
    .expect("write scan fixture");

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept probe");
        assert!(read_request(&mut stream).starts_with("GET /v1/node/info HTTP/1.1\r\n"));
        let body = node_info("builder-scan", &base_url, "direct").encode_to_vec();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/x-protobuf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .expect("write response");
        stream.write_all(&body).expect("write protobuf body");
    });

    let output = run_scan(&config_root, &fixture_path, "enter,tick,tick,enter").expect("run scan");

    assert!(
        output.status.success(),
        "tak remote scan should succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Choose Camera"),
        "missing picker UI:\n{stdout}"
    );
    assert!(
        stdout.contains("Desk Camera"),
        "missing camera name:\n{stdout}"
    );
    assert!(
        stdout.contains("Confirm Remote"),
        "missing confirmation UI:\n{stdout}"
    );
    assert!(
        stdout.contains("builder-scan"),
        "missing decoded node info:\n{stdout}"
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

    server.join().expect("probe server should exit");
}
