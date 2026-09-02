#![cfg(unix)]

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::process::Command;

use crate::support;

#[test]
fn interactive_remote_add_previews_and_confirms_through_protocol_v2() {
    let root = tempfile::tempdir().expect("temp root");
    let socket = root.path().join("takd.sock");
    let bind_path = support::unix_socket_bind_path::short_bind_path(&socket);
    let listener = UnixListener::bind(bind_path).expect("bind daemon");
    let invite = "takd:tor:secret-invite";
    let server = std::thread::spawn(move || {
        for expected in ["PreviewRemote", "AddRemote"] {
            let (mut stream, _) = listener.accept().expect("accept add request");
            let mut request = String::new();
            BufReader::new(stream.try_clone().unwrap())
                .read_line(&mut request)
                .expect("read request");
            let value: serde_json::Value = serde_json::from_str(&request).unwrap();
            assert_eq!(value["operation"]["type"], expected);
            assert_eq!(value["operation"]["invite"], invite);
            let response_type = if expected == "PreviewRemote" {
                "RemotePreview"
            } else {
                "RemoteAdded"
            };
            writeln!(stream, "{{\"protocol_version\":2,\"type\":\"{response_type}\",\"request_id\":{},\"remote\":{{\"node_id\":\"builder-a\",\"display_name\":\"Builder A\",\"base_url\":\"http://builder-a.onion\",\"pools\":[],\"tags\":[],\"capabilities\":[],\"transport\":\"tor\",\"enabled\":true}}}}", value["request_id"]).unwrap();
        }
    });

    let output = Command::new(support::tak_bin())
        .args(["remote", "add"])
        .env("TAKD_SOCKET", &socket)
        .env(
            "TAK_TEST_REMOTE_ADD_SCRIPT",
            format!("down,enter,paste:{invite},enter,enter"),
        )
        .output()
        .expect("remote add");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Confirm Remote"));
    assert!(stdout.contains("added remote builder-a"));
    server.join().expect("daemon exits");
}
