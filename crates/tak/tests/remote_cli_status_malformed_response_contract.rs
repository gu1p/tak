use crate::support;

use std::io::Write;
use std::net::TcpListener;
use std::process::Command as StdCommand;
use std::thread;

use support::remote_cli::read_request;
use support::remote_status::write_inventory;

#[test]
fn remote_status_surfaces_explicit_malformed_response_diagnostics() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind malformed status server");
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
        write!(stream, "not-http\r\n\r\n").expect("write malformed response");
    });

    let output = StdCommand::new(support::tak_bin())
        .args(["remote", "status", "--node", "builder-a"])
        .env("XDG_CONFIG_HOME", &config_root)
        .output()
        .expect("run tak remote status");
    assert!(
        !output.status.success(),
        "tak remote status should fail on malformed HTTP"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(&format!("status=malformed HTTP response from {base_url}")),
        "missing explicit malformed-response status:\n{stdout}"
    );

    server.join().expect("status server should exit");
}
