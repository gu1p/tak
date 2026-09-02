#![cfg(unix)]

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::process::Command;

use crate::support::{self, remote_daemon_v2::FakeRemoteDaemon};

#[test]
fn remote_add_renders_upgrade_guidance_for_direct_v1_invites() {
    let root = tempfile::tempdir().expect("temp root");
    let socket = root.path().join("takd.sock");
    let bind_path = support::unix_socket_bind_path::short_bind_path(&socket);
    let listener = UnixListener::bind(bind_path).expect("bind daemon");
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept add");
        let mut request = String::new();
        BufReader::new(stream.try_clone().unwrap())
            .read_line(&mut request)
            .unwrap();
        let value: serde_json::Value = serde_json::from_str(&request).unwrap();
        assert_eq!(value["operation"]["type"], "AddRemote");
        writeln!(stream, "{{\"protocol_version\":2,\"type\":\"Error\",\"request_id\":{},\"message\":\"Direct v1 onboarding is unsupported. Upgrade tak, takd, and workers together.\",\"code\":\"remote_invite_unsupported\",\"retryable\":false}}", value["request_id"]).unwrap();
    });

    let output = Command::new(support::tak_bin())
        .args(["remote", "add", "takd:v1:legacy-secret"])
        .env("TAKD_SOCKET", &socket)
        .output()
        .expect("remote add");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("upgrade tak, takd, and workers together"),
        "{stderr}"
    );
    server.join().expect("daemon exits");
}

#[test]
fn remote_list_renders_coordinated_upgrade_guidance_for_unexpected_response() {
    let root = tempfile::tempdir().expect("temp root");
    let daemon = FakeRemoteDaemon::spawn(
        root.path(),
        vec![serde_json::json!({
            "type": "RemoteRemoved",
            "node_id": "builder-a",
            "removed": false
        })],
    );

    let output = Command::new(support::tak_bin())
        .args(["remote", "list"])
        .env("TAKD_SOCKET", daemon.socket())
        .output()
        .expect("remote list");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("upgrade tak, takd, and workers together"),
        "{stderr}"
    );
    let requests = daemon.finish();
    assert_eq!(requests[0]["operation"]["type"], "ListRemotes");
}
