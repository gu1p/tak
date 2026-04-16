mod support;

use std::io::Write;
use std::net::TcpListener;
use std::process::Command as StdCommand;
use std::thread;

use prost::Message;
use support::remote_cli::{node_info, read_request, remote_inventory_path};
use tak_proto::encode_tor_invite;

#[test]
fn remote_add_rejects_tor_invite_when_probe_advertises_different_endpoint() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind node info server");
    let port = listener.local_addr().expect("listener addr").port();
    let invited_base_url = "http://builder-a-hidden-service.onion";
    let advertised_base_url = "http://127.0.0.1:43123";
    let invite = encode_tor_invite(invited_base_url).expect("encode tor invite");

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept probe");
        let request = read_request(&mut stream);
        assert!(
            request.starts_with("GET /v1/node/info HTTP/1.1\r\n"),
            "unexpected request: {request}"
        );
        let body = node_info("builder-a", advertised_base_url, "direct").encode_to_vec();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/x-protobuf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .expect("write response");
        stream.write_all(&body).expect("write protobuf body");
    });

    let add = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"))
        .args(["remote", "add", &invite])
        .env("XDG_CONFIG_HOME", &config_root)
        .env("TAK_TEST_TOR_ONION_DIAL_ADDR", format!("127.0.0.1:{port}"))
        .output()
        .expect("run tak remote add");

    let stderr = String::from_utf8_lossy(&add.stderr);
    assert!(!add.status.success(), "tak remote add should fail");
    assert!(
        stderr.contains(
            "tor invite expected http://builder-a-hidden-service.onion via tor, probe returned http://127.0.0.1:43123 via direct"
        ),
        "unexpected stderr:\n{stderr}"
    );
    assert!(
        !remote_inventory_path(&config_root).exists(),
        "remote inventory should stay empty"
    );

    server.join().expect("probe server should exit");
}
