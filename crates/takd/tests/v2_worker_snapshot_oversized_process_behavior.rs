use std::process::Command;

use tak_proto::worker_v2::decode_snapshot;

use crate::support::{
    v2_worker_http::{get, status},
    worker_http::start_server,
};

#[tokio::test]
async fn oversized_host_argv_keeps_snapshot_available_with_an_incomplete_marker() {
    let marker = format!(
        "tak-worker-oversized-{}{}",
        uuid::Uuid::new_v4(),
        "x".repeat(20_000)
    );
    let mut process = Command::new("/bin/sh")
        .args(["-c", "trap ':' EXIT; sleep 30", marker.as_str()])
        .spawn()
        .unwrap();
    let server = start_server().await;

    let response = get(&server, "/v2/worker/snapshot", Some("secret"), &["v2"]).await;
    process.kill().unwrap();
    process.wait().unwrap();

    assert_eq!(status(&response), 200, "{}", response.head);
    let snapshot = decode_snapshot(&response.body).unwrap();
    assert!(
        snapshot
            .processes
            .iter()
            .any(|process| process.name == "process-observations-incomplete")
    );
}
