use crate::support;

use std::fs;
use std::io::Write;
use std::net::TcpListener;
use std::process::Command as StdCommand;
use std::thread;

use support::remote_cli::read_request;
use support::remote_status::status_payload_with_detail_for;

#[test]
fn remote_status_allows_empty_bearer_token_for_tor_transport() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind tor status server");
    let port = listener.local_addr().expect("listener addr").port();
    let base_url = format!("http://127.0.0.1:{port}");
    let server_base_url = base_url.clone();
    let inventory_path = config_root.join("tak/remotes.toml");
    fs::create_dir_all(inventory_path.parent().expect("inventory parent")).expect("config root");
    fs::write(
        &inventory_path,
        format!(
            "version = 1\n\n[[remotes]]\nnode_id = \"builder-status\"\ndisplay_name = \"builder-status\"\nbase_url = \"{base_url}\"\nbearer_token = \"\"\npools = [\"default\"]\ntags = [\"builder\"]\ncapabilities = [\"linux\"]\ntransport = \"tor\"\nenabled = true\n"
        ),
    )
    .expect("write inventory");

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept tor status request");
        let request = read_request(&mut stream);
        assert!(
            request.starts_with("GET /v1/node/status HTTP/1.1\r\n"),
            "unexpected request: {request}"
        );
        assert!(
            !request.contains("Authorization:"),
            "tor status requests should omit auth when bearer is empty:\n{request}"
        );
        let body = status_payload_with_detail_for(
            "builder-status",
            &server_base_url,
            "tor",
            false,
            "tor status detail is preserved",
        );
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/x-protobuf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .expect("write response head");
        stream.write_all(&body).expect("write response body");
    });

    let output = StdCommand::new(support::tak_bin())
        .args(["remote", "status", "--node", "builder-status"])
        .env("XDG_CONFIG_HOME", &config_root)
        .env("TAKD_SOCKET", temp.path().join(".missing-takd.sock"))
        .output()
        .expect("run tak remote status for onion transport");
    assert!(
        output.status.success(),
        "tak remote status should succeed with tor url-only auth\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("detail=tor status detail is preserved"),
        "missing full detail:\n{stdout}"
    );

    server.join().expect("status server should exit");
}
