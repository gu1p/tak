use crate::support;

use std::io::Write;
use std::net::TcpListener;
use std::process::Command as StdCommand;
use std::thread;

use support::remote_cli::read_request;
use support::remote_status::write_inventory_entries;

#[test]
fn status_with_selected_remote_http_failure_exits_non_zero() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let state_root = temp.path().join("state");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind failing node");
    let addr = listener.local_addr().expect("node addr");
    let base_url = format!("http://{addr}");
    write_inventory_entries(&config_root, &[("builder-z", &base_url, "direct", true)]);

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept status request");
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
        .args(["status", "--node", "builder-z"])
        .env("XDG_CONFIG_HOME", &config_root)
        .env("XDG_STATE_HOME", &state_root)
        .env("TAKD_SOCKET", temp.path().join(".missing-takd.sock"))
        .output()
        .expect("run tak status");
    server.join().expect("status server should exit");

    assert!(
        !output.status.success(),
        "tak status should fail when selected remote status fails\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Local"), "missing local section:\n{stdout}");
    assert!(
        stdout.contains("Remote Nodes"),
        "missing remote section:\n{stdout}"
    );
    assert!(
        stdout.contains("builder-z"),
        "missing failing node:\n{stdout}"
    );
    assert!(
        stdout.contains("status=node status failed with HTTP 401"),
        "missing HTTP failure detail:\n{stdout}"
    );
}
