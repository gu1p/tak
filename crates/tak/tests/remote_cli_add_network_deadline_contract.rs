#![cfg(unix)]

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::process::Command;
use std::time::{Duration, Instant};

use crate::support;

#[test]
fn remote_add_allows_a_bounded_daemon_owned_network_probe_beyond_two_seconds() {
    let root = tempfile::tempdir().expect("temp root");
    let socket = root.path().join("takd.sock");
    let bind_path = support::unix_socket_bind_path::short_bind_path(&socket);
    let listener = UnixListener::bind(bind_path).expect("bind daemon");
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept add request");
        let mut request = String::new();
        BufReader::new(stream.try_clone().unwrap())
            .read_line(&mut request)
            .expect("read request");
        let value: serde_json::Value = serde_json::from_str(&request).unwrap();
        assert_eq!(value["operation"]["type"], "AddRemote");
        std::thread::sleep(Duration::from_millis(2_100));
        writeln!(stream, "{{\"protocol_version\":2,\"type\":\"RemoteAdded\",\"request_id\":{},\"remote\":{{\"node_id\":\"builder-a\",\"display_name\":\"Builder A\",\"base_url\":\"http://builder-a.onion\",\"pools\":[],\"tags\":[],\"capabilities\":[],\"transport\":\"tor\",\"enabled\":true}}}}", value["request_id"]).expect("write response");
    });

    let started = Instant::now();
    let output = Command::new(support::tak_bin())
        .args(["remote", "add", "takd:tor:secret-invite"])
        .env("TAKD_SOCKET", &socket)
        .output()
        .expect("remote add");

    assert!(started.elapsed() < Duration::from_secs(5));
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("added remote builder-a"));
    server.join().expect("daemon exits");
}
