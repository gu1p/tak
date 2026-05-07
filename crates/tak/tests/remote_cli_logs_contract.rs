use crate::support;

use std::io::Write;
use std::net::TcpListener;
use std::process::Command as StdCommand;
use std::thread;

use support::remote_cli::read_request;
use support::remote_status::write_inventory;

#[test]
fn remote_logs_fetches_complete_service_log_for_selected_node() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind logs server");
    let addr = listener.local_addr().expect("listener addr");
    let base_url = format!("http://{addr}");
    write_inventory(&config_root, "builder-a", &base_url);

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept logs request");
        let request = read_request(&mut stream);
        assert!(
            request.starts_with("GET /v1/node/logs?all=true HTTP/1.1\r\n"),
            "unexpected request: {request}"
        );
        let body = b"booting takd\nremote service ready\n";
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .expect("write response head");
        stream.write_all(body).expect("write response body");
    });

    let output = StdCommand::new(support::tak_bin())
        .args(["remote", "logs", "--node", "builder-a", "--all"])
        .env("XDG_CONFIG_HOME", &config_root)
        .output()
        .expect("run tak remote logs");

    assert!(
        output.status.success(),
        "tak remote logs should succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "booting takd\nremote service ready\n"
    );
    server.join().expect("logs server should exit");
}
