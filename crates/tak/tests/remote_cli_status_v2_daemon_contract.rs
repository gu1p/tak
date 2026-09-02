#![cfg(unix)]

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::process::Command;

use crate::support;

#[test]
fn remote_status_preserves_rendering_while_health_comes_from_protocol_v2() {
    let root = tempfile::tempdir().expect("temp root");
    let socket = root.path().join("takd.sock");
    let bind_path = support::unix_socket_bind_path::short_bind_path(&socket);
    let listener = UnixListener::bind(bind_path).expect("bind daemon");
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept status");
        let mut request = String::new();
        BufReader::new(stream.try_clone().unwrap())
            .read_line(&mut request)
            .unwrap();
        let value: serde_json::Value = serde_json::from_str(&request).unwrap();
        assert_eq!(value["operation"]["type"], "GetRemoteStatus");
        assert_eq!(
            value["operation"]["node_ids"],
            serde_json::json!(["builder-a"])
        );
        writeln!(stream, "{{\"protocol_version\":2,\"type\":\"RemoteStatus\",\"request_id\":{},\"remotes\":[{{\"remote\":{{\"node_id\":\"builder-a\",\"display_name\":\"Builder A\",\"base_url\":\"http://127.0.0.1:9\",\"pools\":[],\"tags\":[],\"capabilities\":[],\"transport\":\"direct\",\"enabled\":true}},\"snapshot\":{{\"protocol_version\":2,\"node_id\":\"builder-a\",\"healthy\":true,\"sampled_at_ms\":1,\"capacity\":{{\"cpu_millis\":8000,\"memory_bytes\":16000,\"execution_slots\":8}},\"usage\":{{\"cpu_millis\":1000,\"memory_bytes\":4000,\"execution_slots\":2}},\"queue_depth\":1,\"cached_content\":[],\"processes\":[{{\"name\":\"private-process\",\"arguments\":[\"--token=private-value\"]}}]}},\"detail_base64\":null,\"error\":null,\"peer\":null}}]}}", value["request_id"]).unwrap();
    });

    let output = Command::new(support::tak_bin())
        .args(["remote", "status", "--node", "builder-a"])
        .env("TAKD_SOCKET", &socket)
        .output()
        .expect("remote status");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("builder-a transport=direct state=ready"),
        "{stdout}"
    );
    assert!(stdout.contains("Active Jobs"));
    assert!(!stdout.contains("private-process"));
    assert!(!stdout.contains("private-value"));
    server.join().expect("daemon exits");
}
