#![cfg(unix)]

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::process::Command;

use crate::support;

#[test]
fn remote_list_preserves_output_while_reading_inventory_from_protocol_v2() {
    let root = tempfile::tempdir().expect("temp root");
    let socket = root.path().join("takd.sock");
    let bind_path = support::unix_socket_bind_path::short_bind_path(&socket);
    let listener = UnixListener::bind(bind_path).expect("bind daemon");
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept list");
        let mut request = String::new();
        BufReader::new(stream.try_clone().unwrap())
            .read_line(&mut request)
            .expect("read request");
        assert!(request.contains(r#""type":"ListRemotes""#), "{request}");
        let id = serde_json::from_str::<serde_json::Value>(&request).unwrap()["request_id"]
            .as_str()
            .unwrap()
            .to_string();
        writeln!(stream, "{{\"protocol_version\":2,\"type\":\"RemoteList\",\"request_id\":\"{id}\",\"remotes\":[{{\"node_id\":\"builder-a\",\"display_name\":\"Builder A\",\"base_url\":\"http://builder-a.onion\",\"pools\":[\"build\"],\"tags\":[\"linux\"],\"capabilities\":[\"docker\"],\"transport\":\"tor\",\"enabled\":true}}]}}").unwrap();
    });

    let output = Command::new(support::tak_bin())
        .args(["remote", "list"])
        .env("TAKD_SOCKET", &socket)
        .env("XDG_CONFIG_HOME", root.path().join("unread-config"))
        .output()
        .expect("remote list");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Configured remote inventory (reachability not checked)"));
    assert!(stdout.contains("builder-a"));
    assert!(stdout.contains("http://builder-a.onion"));
    server.join().expect("daemon exits");
}
