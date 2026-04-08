mod support;

use std::io::Write;
use std::net::TcpListener;
use std::process::Command as StdCommand;
use std::thread;

use support::remote_cli::read_request;
use support::remote_status::{status_payload, write_inventory};

#[test]
fn remote_status_watch_refreshes_until_test_limit() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind watch status server");
    let addr = listener.local_addr().expect("listener addr");
    let base_url = format!("http://{addr}");
    write_inventory(&config_root, "builder-a", &base_url);

    let server = thread::spawn(move || {
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().expect("accept status request");
            let request = read_request(&mut stream);
            assert!(
                request.starts_with("GET /v1/node/status HTTP/1.1\r\n"),
                "unexpected request: {request}"
            );
            let body = status_payload(&base_url, false);
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/x-protobuf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .expect("write response head");
            stream.write_all(&body).expect("write response body");
        }
    });

    let output = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"))
        .args(["remote", "status", "--watch", "--interval-ms", "1"])
        .env("XDG_CONFIG_HOME", &config_root)
        .env("TAK_TEST_REMOTE_STATUS_MAX_POLLS", "2")
        .output()
        .expect("run tak remote status --watch");
    assert!(
        output.status.success(),
        "tak remote status --watch should succeed"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.matches("Nodes").count() >= 2,
        "watch output should render multiple snapshots:\n{stdout}"
    );
    server.join().expect("status server should exit");
}
