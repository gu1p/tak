mod support;

use std::io::Write;
use std::net::TcpListener;
use std::process::Command as StdCommand;
use std::thread;

use support::remote_cli::read_request;
use support::remote_status::{status_payload_for, write_inventory_entries};

#[test]
fn remote_status_uses_direct_http_for_tor_transport_without_onion_host() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind tor-over-http node");
    let addr = listener.local_addr().expect("listener addr");
    let base_url = format!("http://{addr}");
    write_inventory_entries(&config_root, &[("builder-tor", &base_url, "tor", true)]);

    let server = thread::spawn(move || {
        let (mut stream, _) = listener
            .accept()
            .expect("accept tor-over-http status request");
        let request = read_request(&mut stream);
        assert!(
            request.starts_with("GET /v1/node/status HTTP/1.1\r\n"),
            "unexpected request: {request}"
        );
        let body = status_payload_for("builder-tor", &base_url, "tor", false);
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/x-protobuf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .expect("write response head");
        stream.write_all(&body).expect("write response body");
    });

    let output = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"))
        .args(["remote", "status", "--node", "builder-tor"])
        .env("XDG_CONFIG_HOME", &config_root)
        .output()
        .expect("run tak remote status with tor transport");
    assert!(
        output.status.success(),
        "tak remote status should allow direct HTTP for non-onion tor nodes"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("builder-tor transport=tor"),
        "missing tor transport row:\n{stdout}"
    );
    assert!(
        stdout.contains("Active Jobs\n(none)\n"),
        "expected empty active jobs output:\n{stdout}"
    );
    server.join().expect("status server should exit");
}
