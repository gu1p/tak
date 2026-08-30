#![cfg(unix)]

use std::time::Duration;

use tak_proto::local_daemon::v2::{DaemonErrorCode, ErrorResponse, Operation, Request, Response};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::time::{pause, resume, sleep};

use super::client::send_response;

#[tokio::test(flavor = "current_thread")]
async fn a_fragmented_response_can_complete_after_a_short_daemon_delay() {
    let root = tempfile::tempdir().expect("temp root");
    let socket = root.path().join("takd.sock");
    let listener = UnixListener::bind(&socket).expect("bind fake daemon");
    let request_id = "delayed-list";
    let response = serde_json::to_vec(&ErrorResponse::v2_not_active(request_id.into()))
        .expect("encode response");
    let split = response.len() / 2;
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept request");
        let mut reader = BufReader::new(stream);
        let mut request = String::new();
        reader.read_line(&mut request).await.expect("read request");
        assert!(request.ends_with('\n'));

        let mut stream = reader.into_inner();
        stream
            .write_all(&response[..split])
            .await
            .expect("write response prefix");
        pause();
        sleep(Duration::from_millis(100)).await;
        resume();
        stream
            .write_all(&response[split..])
            .await
            .expect("write response suffix");
        stream.write_all(b"\n").await.expect("finish response");
    });
    let request = Request {
        request_id: request_id.into(),
        operation: Operation::ListRuns {},
    };

    let result = send_response(&socket, &request).await;

    assert!(matches!(
        result,
        Ok(Response::Error {
            code: DaemonErrorCode::ProtocolV2NotActive,
            ..
        })
    ));
    server.await.expect("join fake daemon");
}
