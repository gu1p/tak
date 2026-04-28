use std::io::Write;
use std::net::TcpListener;
use std::process::Command as StdCommand;
use std::thread;

use crate::support;
use crate::support::remote_cli::read_request;
use crate::support::remote_status::{status_payload_with_image_cache, write_inventory};

#[test]
fn remote_status_renders_image_cache_telemetry() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind node status server");
    let addr = listener.local_addr().expect("listener addr");
    let base_url = format!("http://{addr}");
    write_inventory(&config_root, "builder-a", &base_url);

    let server = thread::spawn({
        let base_url = base_url.clone();
        move || {
            let (mut stream, _) = listener.accept().expect("accept status request");
            let request = read_request(&mut stream);
            assert!(
                request.starts_with("GET /v1/node/status HTTP/1.1\r\n"),
                "unexpected request: {request}"
            );
            let body = status_payload_with_image_cache(&base_url);
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/x-protobuf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .expect("write response head");
            stream.write_all(&body).expect("write response body");
        }
    });

    let output = StdCommand::new(support::tak_bin())
        .args(["remote", "status", "--node", "builder-a"])
        .env("XDG_CONFIG_HOME", &config_root)
        .output()
        .expect("run tak remote status");
    assert!(output.status.success(), "tak remote status should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("image_cache=12.4GB/50.0GB"),
        "missing image cache usage:\n{stdout}"
    );
    assert!(
        stdout.contains("image_cache_entries=7"),
        "missing image cache entry count:\n{stdout}"
    );
    server.join().expect("status server should exit");
}
