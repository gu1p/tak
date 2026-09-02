use takd::SubmitAttemptStore;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

use crate::support::remote_output::test_context;

const UPGRADE: &str = "upgrade tak, takd, and workers together";

#[tokio::test(flavor = "multi_thread")]
async fn local_v1_execution_requests_are_rejected_with_upgrade_guidance() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let relative_root = std::path::Path::new(".tmp").join(temp.path().file_name().unwrap());
    let socket = relative_root.join("d.sock");
    let server = crate::support::protocol_server::spawn_protocol_server(
        temp.path().join("takd.sqlite"),
        socket.clone(),
    );
    let mut daemon = BufReader::new(connect(&socket).await);

    daemon
        .get_mut()
        .write_all(concat!(
            r#"{"type":"StreamTaskEvents","request_id":"legacy-events","task_handle":"remote:worker:job","after_seq":0}"#,
            "\n"
        ).as_bytes())
        .await
        .unwrap();
    let mut response = String::new();
    daemon.read_line(&mut response).await.unwrap();
    let response: serde_json::Value = serde_json::from_str(&response).expect("response json");

    assert_eq!(response["protocol_version"], 2);
    assert_eq!(response["code"], "protocol_version_unsupported");
    assert!(
        response["message"]
            .as_str()
            .unwrap()
            .to_ascii_lowercase()
            .contains(UPGRADE)
    );
    server.abort();
}

#[test]
fn legacy_submit_is_rejected_before_payload_decoding() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).unwrap();

    let response = takd::daemon::remote::handle_worker_http_request(
        &test_context(),
        &store,
        "POST",
        "/v1/tasks/submit",
        &[],
        Some(b"not-a-v1-payload"),
    )
    .unwrap();

    assert_eq!(response.status_code, 426);
    assert!(String::from_utf8_lossy(&response.body).contains(UPGRADE));
}

async fn connect(socket: &std::path::Path) -> UnixStream {
    let connection_path = crate::support::socket_path::bind_path(socket);
    for _ in 0..100 {
        if let Ok(stream) = UnixStream::connect(&connection_path).await {
            return stream;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    panic!("daemon socket did not become ready: {}", socket.display());
}
