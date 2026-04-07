//! Contract tests for client-managed remote inventory.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command as StdCommand;
use std::thread;

use prost::Message;
use tak_proto::{NodeInfo, RemoteTokenPayload, encode_remote_token};

#[test]
fn remote_add_imports_takd_token_and_lists_remote() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind node info server");
    let port = listener.local_addr().expect("listener addr").port();
    let base_url = "http://builder-a-hidden-service.onion".to_string();
    let server_base_url = base_url.clone();

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept probe");
        let mut request = Vec::new();
        let mut buf = [0_u8; 256];
        loop {
            let read = stream.read(&mut buf).expect("read request");
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buf[..read]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        let request = String::from_utf8(request).expect("request utf8");
        assert!(request.starts_with("GET /v1/node/info HTTP/1.1\r\n"));
        let body = NodeInfo {
            node_id: "builder-a".into(),
            display_name: "builder-a".into(),
            base_url: server_base_url,
            healthy: true,
            pools: vec!["default".into()],
            tags: vec!["builder".into()],
            capabilities: vec!["linux".into()],
            transport: "tor".into(),
        }
        .encode_to_vec();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/x-protobuf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .expect("write response");
        stream.write_all(&body).expect("write protobuf body");
    });

    let token = encode_remote_token(&RemoteTokenPayload {
        version: "v1".into(),
        node: Some(NodeInfo {
            node_id: "builder-a".into(),
            display_name: "builder-a".into(),
            base_url: base_url.clone(),
            healthy: true,
            pools: vec!["default".into()],
            tags: vec!["builder".into()],
            capabilities: vec!["linux".into()],
            transport: "tor".into(),
        }),
        bearer_token: "secret".into(),
    })
    .expect("encode remote token");

    let add = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"))
        .args(["remote", "add", &token])
        .env("XDG_CONFIG_HOME", &config_root)
        .env("TAK_TEST_TOR_ONION_DIAL_ADDR", format!("127.0.0.1:{port}"))
        .output()
        .expect("run tak remote add");
    assert!(
        add.status.success(),
        "tak remote add should succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&add.stdout),
        String::from_utf8_lossy(&add.stderr)
    );

    let list = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"))
        .args(["remote", "list"])
        .env("XDG_CONFIG_HOME", &config_root)
        .output()
        .expect("run tak remote list");
    assert!(list.status.success(), "tak remote list should succeed");
    let stdout = String::from_utf8_lossy(&list.stdout);
    assert!(
        stdout.contains("builder-a"),
        "missing node id in list: {stdout}"
    );
    assert!(stdout.contains("default"), "missing pool in list: {stdout}");

    server.join().expect("probe server should exit");
}
