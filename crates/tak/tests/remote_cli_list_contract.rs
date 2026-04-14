mod support;

use std::io::Write;
use std::net::TcpListener;
use std::process::Command as StdCommand;
use std::thread;

use prost::Message;
use support::remote_cli::{node_info, read_request};
use tak_proto::{RemoteTokenPayload, encode_remote_token};

#[test]
fn remote_add_imports_takd_token_and_lists_remote() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind node info server");
    let port = listener.local_addr().expect("listener addr").port();
    let base_url = "http://builder-a-hidden-service.onion";
    let token = encode_remote_token(&RemoteTokenPayload {
        version: "v1".into(),
        node: Some(tak_proto::NodeInfo {
            pools: vec!["stale".into()],
            ..node_info("builder-a", base_url, "tor")
        }),
        bearer_token: "secret".into(),
    })
    .expect("encode remote token");

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept probe");
        assert!(read_request(&mut stream).starts_with("GET /v1/node/info HTTP/1.1\r\n"));
        let body = node_info("builder-a", base_url, "tor").encode_to_vec();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/x-protobuf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .expect("write response");
        stream.write_all(&body).expect("write protobuf body");
    });

    let add = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"))
        .args(["remote", "add", &token])
        .env("XDG_CONFIG_HOME", &config_root)
        .env("TAK_TEST_TOR_ONION_DIAL_ADDR", format!("127.0.0.1:{port}"))
        .output()
        .expect("run tak remote add");
    assert!(add.status.success(), "tak remote add should succeed");

    let list = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"))
        .args(["remote", "list"])
        .env("XDG_CONFIG_HOME", &config_root)
        .output()
        .expect("run tak remote list");
    let stdout = String::from_utf8_lossy(&list.stdout);
    assert!(list.status.success(), "tak remote list should succeed");
    assert!(
        stdout.contains("builder-a"),
        "missing node id in list: {stdout}"
    );
    assert!(stdout.contains("default"), "missing pool in list: {stdout}");
    assert!(
        !stdout.contains("stale"),
        "list should use probed node info:\n{stdout}"
    );

    server.join().expect("probe server should exit");
}
