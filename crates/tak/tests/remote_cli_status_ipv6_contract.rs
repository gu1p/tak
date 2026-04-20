use crate::support;

use std::io::Write;
use std::net::TcpListener;
use std::process::Command as StdCommand;
use std::thread;

use support::remote_cli::read_request;
use support::remote_status::{status_payload, write_inventory};

#[test]
fn remote_status_reaches_direct_ipv6_nodes() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let listener = match TcpListener::bind("[::1]:0") {
        Ok(listener) => listener,
        Err(err) if err.kind() == std::io::ErrorKind::AddrNotAvailable => return,
        Err(err) => panic!("bind ipv6 node status server: {err}"),
    };
    let addr = listener.local_addr().expect("listener addr");
    let base_url = format!("http://{addr}");
    write_inventory(&config_root, "builder-ipv6", &base_url);

    let server = thread::spawn(move || {
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
    });

    let output = StdCommand::new(support::tak_bin())
        .args(["remote", "status", "--node", "builder-ipv6"])
        .env("XDG_CONFIG_HOME", &config_root)
        .output()
        .expect("run tak remote status");
    assert!(
        output.status.success(),
        "tak remote status should succeed for ipv6 remotes\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    server.join().expect("status server should exit");
}
