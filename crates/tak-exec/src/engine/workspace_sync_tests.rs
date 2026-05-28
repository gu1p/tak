#![cfg(test)]

use std::fs;

use super::workspace_sync::sync_remote_outputs_from_remote;
use super::workspace_sync_test_support::{RangeServer, direct_target, synced_output};

#[tokio::test]
async fn remote_output_download_retries_dropped_range_response() {
    let temp = tempfile::tempdir().expect("tempdir");
    let body = b"hello resumable".to_vec();
    fs::write(temp.path().join("out.txt.tak-part"), b"hello ").expect("partial");
    let server = RangeServer::spawn(body.clone()).await;
    let target = direct_target(&server.addr);
    let output = synced_output("out.txt", &body);

    sync_remote_outputs_from_remote(&target, "run-1", 1, temp.path(), &[output])
        .await
        .expect("sync");

    assert_eq!(fs::read(temp.path().join("out.txt")).expect("output"), body);
    assert_eq!(server.ranges().await, vec!["bytes=6-14", "bytes=6-14"]);
    assert!(server.dropped_response());
}

#[tokio::test]
async fn remote_output_download_discards_stale_partial_after_digest_mismatch() {
    let temp = tempfile::tempdir().expect("tempdir");
    let body = b"fresh-output".to_vec();
    let stale = b"wrong-output";
    let partial_path = temp.path().join("out.txt.tak-part");
    fs::write(&partial_path, stale).expect("partial");
    let server = RangeServer::spawn_without_drop(body.clone()).await;
    let target = direct_target(&server.addr);
    let output = synced_output("out.txt", &body);

    let first = sync_remote_outputs_from_remote(
        &target,
        "run-1",
        1,
        temp.path(),
        std::slice::from_ref(&output),
    )
    .await;

    assert!(format!("{:#}", first.expect_err("stale partial should fail")).contains("digest"));
    assert!(!partial_path.exists());

    sync_remote_outputs_from_remote(&target, "run-1", 1, temp.path(), &[output])
        .await
        .expect("retry after stale partial cleanup");

    assert_eq!(fs::read(temp.path().join("out.txt")).expect("output"), body);
    assert_eq!(server.ranges().await, vec!["bytes=0-11".to_string()]);
}

#[tokio::test]
async fn remote_output_download_requests_bounded_chunks_and_streams_final_digest() {
    let temp = tempfile::tempdir().expect("tempdir");
    let body = vec![b'x'; 8 * 1024 * 1024 + 3];
    let server = RangeServer::spawn_without_drop(body.clone()).await;
    let target = direct_target(&server.addr);
    let output = synced_output("large.bin", &body);

    sync_remote_outputs_from_remote(&target, "run-1", 1, temp.path(), &[output])
        .await
        .expect("sync large output");

    assert_eq!(
        fs::metadata(temp.path().join("large.bin")).unwrap().len(),
        body.len() as u64
    );
    assert_eq!(
        server.ranges().await,
        vec![
            "bytes=0-8388607".to_string(),
            "bytes=8388608-8388610".to_string()
        ]
    );
}
