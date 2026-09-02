use tak_proto::worker_v2::{
    WorkspaceCacheDisposition, decode_snapshot, encode_dispatch_request,
};

use crate::support::{
    worker_http::start_server,
    v2_worker_cache::{probe, upload},
    v2_worker_execution::{output_archive, output_dispatch},
    v2_worker_http::{get, post, status},
};

#[tokio::test]
async fn worker_cache_reports_miss_then_atomically_reuses_one_verified_blob() {
    let server = start_server().await;
    let request = output_dispatch();
    let descriptor = request.payload.workspace.descriptor.clone();
    assert_eq!(
        probe(&server, &descriptor).await.disposition,
        WorkspaceCacheDisposition::Miss
    );
    let missing = post(
        &server,
        "/v2/attempts/dispatch",
        Some("secret"),
        &["v2"],
        &encode_dispatch_request(&request).unwrap(),
    )
    .await;
    assert_eq!(
        status(&missing),
        412,
        "{} {}",
        missing.head,
        String::from_utf8_lossy(&missing.body)
    );
    let archive = output_archive();
    let (first, second) = tokio::join!(
        upload(&server, &descriptor, &archive),
        upload(&server, &descriptor, &archive)
    );
    assert!(matches!(
        (first.disposition, second.disposition),
        (WorkspaceCacheDisposition::Stored, WorkspaceCacheDisposition::Hit)
            | (WorkspaceCacheDisposition::Hit, WorkspaceCacheDisposition::Stored)
    ));
    assert_eq!(
        probe(&server, &descriptor).await.disposition,
        WorkspaceCacheDisposition::Hit
    );
    let response = get(
        &server,
        "/v2/worker/snapshot",
        Some("secret"),
        &["v2"],
    )
    .await;
    assert_eq!(status(&response), 200);
    let snapshot = decode_snapshot(&response.body).unwrap();
    assert_eq!(snapshot.cached_content, [descriptor.manifest.fingerprint]);
}
