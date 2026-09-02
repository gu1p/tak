use crate::support;
use support::local_broker_http::send_raw_http;
use support::recording_remote::RecordingRemote;

#[tokio::test(flavor = "multi_thread")]
async fn raw_http_on_the_local_daemon_socket_cannot_direct_the_remote_broker() {
    let temp = tempfile::tempdir().expect("tempdir");
    let remote = RecordingRemote::spawn("attacker-selected").await;
    let socket_path = temp.path().join("run/takd.sock");
    let server_socket_path = socket_path.clone();
    let broker = crate::support::local_runtime::tor_broker(remote.addr.clone());
    let server = crate::support::local_runtime::spawn_local_server(server_socket_path, broker);
    let request = format!(
        "GET /v2/worker/identity HTTP/1.1\r\nHost: attacker.invalid\r\nX-Tak-Remote-Node: attacker-selected\r\nX-Tak-Remote-Endpoint: http://{}\r\nX-Tak-Remote-Transport: tor\r\nConnection: close\r\n\r\n",
        remote.addr
    );

    let response = send_raw_http(&socket_path, request.as_bytes()).await;

    let response = String::from_utf8_lossy(&response);
    assert!(response.contains(r#""code":"protocol_version_unsupported""#));
    assert!(response.contains("Upgrade tak, takd, and workers together"));
    assert_eq!(remote.request_count(), 0);
    server.abort();
}
