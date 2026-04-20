use crate::support;

use std::io::Write;
use std::net::TcpListener;
use std::process::Command as StdCommand;
use std::thread;

use support::remote_cli::read_request;
use support::remote_status::{status_payload, write_inventory};

#[test]
fn remote_status_lists_running_jobs_and_resources() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind node status server");
    let addr = listener.local_addr().expect("listener addr");
    let base_url = format!("http://{addr}");
    write_inventory(&config_root, "builder-a", &base_url);

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept status request");
        let request = read_request(&mut stream);
        assert!(
            request.starts_with("GET /v1/node/status HTTP/1.1\r\n"),
            "unexpected request: {request}"
        );
        let body = status_payload(&base_url, true);
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/x-protobuf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .expect("write response head");
        stream.write_all(&body).expect("write response body");
    });

    let output = StdCommand::new(support::tak_bin())
        .args(["remote", "status", "--node", "builder-a"])
        .env("XDG_CONFIG_HOME", &config_root)
        .output()
        .expect("run tak remote status");
    assert!(output.status.success(), "tak remote status should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Nodes"), "missing nodes section:\n{stdout}");
    assert!(stdout.contains("builder-a"), "missing node id:\n{stdout}");
    assert!(
        stdout.contains("state=ready"),
        "missing transport state:\n{stdout}"
    );
    assert!(stdout.contains("cpu="), "missing cpu usage:\n{stdout}");
    assert!(stdout.contains("ram="), "missing ram usage:\n{stdout}");
    assert!(
        stdout.contains("storage="),
        "missing storage usage:\n{stdout}"
    );
    assert!(
        stdout.contains("Active Jobs"),
        "missing jobs section:\n{stdout}"
    );
    assert!(
        stdout.contains("//apps/web:build"),
        "missing active task label:\n{stdout}"
    );
    server.join().expect("status server should exit");
}
