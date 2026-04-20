use crate::support;

use std::io::Write;
use std::net::TcpListener;
use std::process::Command as StdCommand;
use std::thread;

use support::remote_cli::read_request;
use support::remote_status::{status_payload_for, write_inventory_entries};

#[test]
fn remote_status_renders_partial_http_failures_and_exits_non_zero() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let ok_listener = TcpListener::bind("127.0.0.1:0").expect("bind ok node");
    let ok_addr = ok_listener.local_addr().expect("ok addr");
    let ok_base_url = format!("http://{ok_addr}");
    let err_listener = TcpListener::bind("127.0.0.1:0").expect("bind failing node");
    let err_addr = err_listener.local_addr().expect("err addr");
    let err_base_url = format!("http://{err_addr}");
    write_inventory_entries(
        &config_root,
        &[
            ("builder-z", &err_base_url, "direct", true),
            ("builder-a", &ok_base_url, "direct", true),
        ],
    );

    let ok_server = thread::spawn(move || {
        let (mut stream, _) = ok_listener.accept().expect("accept ok status request");
        let request = read_request(&mut stream);
        assert!(
            request.starts_with("GET /v1/node/status HTTP/1.1\r\n"),
            "unexpected request: {request}"
        );
        let body = status_payload_for("builder-a", &ok_base_url, "direct", true);
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/x-protobuf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .expect("write ok response head");
        stream.write_all(&body).expect("write ok response body");
    });
    let err_server = thread::spawn(move || {
        let (mut stream, _) = err_listener
            .accept()
            .expect("accept failing status request");
        let request = read_request(&mut stream);
        assert!(
            request.starts_with("GET /v1/node/status HTTP/1.1\r\n"),
            "unexpected request: {request}"
        );
        write!(
            stream,
            "HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        )
        .expect("write failing response");
    });

    let output = StdCommand::new(support::tak_bin())
        .args(["remote", "status"])
        .env("XDG_CONFIG_HOME", &config_root)
        .output()
        .expect("run tak remote status");
    assert!(
        !output.status.success(),
        "tak remote status should fail when one node errors"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("builder-a"), "missing ok node:\n{stdout}");
    assert!(
        stdout.contains("builder-z"),
        "missing failing node:\n{stdout}"
    );
    assert!(
        stdout.contains("status=ok"),
        "missing success row in snapshot:\n{stdout}"
    );
    assert!(
        stdout.contains("status=node status failed with HTTP 401"),
        "missing HTTP failure detail:\n{stdout}"
    );
    assert!(
        stdout.find("builder-a").expect("builder-a row")
            < stdout.find("builder-z").expect("builder-z row"),
        "rows should be sorted by node id:\n{stdout}"
    );
    ok_server.join().expect("ok status server should exit");
    err_server
        .join()
        .expect("failing status server should exit");
}
