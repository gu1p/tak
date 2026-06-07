use prost::Message;
use tak_proto::{ErrorResponse, StartWorkspaceWormholeUploadResponse};
use takd::{RemoteRuntimeConfig, SubmitAttemptStore, handle_remote_v1_request};

#[test]
fn workspace_wormhole_route_advertises_support() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let runtime = RemoteRuntimeConfig::for_tests().with_temp_dir(temp.path());
    let context = crate::support::remote_output::test_context_with_runtime(runtime)
        .with_state_root(temp.path());

    let response = handle_remote_v1_request(
        &context,
        &store,
        "GET",
        "/v2/workspaces/uploads/run-1-1-digest/wormhole",
        None,
    )
    .expect("wormhole route");

    assert_eq!(response.status_code, 200);
    assert!(response.headers.iter().any(|(name, value)| {
        name.eq_ignore_ascii_case("x-tak-workspace-transfer") && value == "wormhole"
    }));
    let available =
        StartWorkspaceWormholeUploadResponse::decode(response.body.as_slice()).expect("decode");
    assert_eq!(available.upload_id, "run-1-1-digest");
    assert_eq!(available.size_bytes, 0);
    assert!(!available.complete);
}

#[test]
fn workspace_wormhole_receive_rejects_plain_router_post() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let runtime = RemoteRuntimeConfig::for_tests().with_temp_dir(temp.path());
    let context = crate::support::remote_output::test_context_with_runtime(runtime)
        .with_state_root(temp.path());

    let response = handle_remote_v1_request(
        &context,
        &store,
        "POST",
        "/v2/workspaces/uploads/run-1-1-digest/wormhole",
        Some(&[]),
    )
    .expect("wormhole post");

    assert_eq!(response.status_code, 426);
    let error = ErrorResponse::decode(response.body.as_slice()).expect("decode error");
    assert_eq!(error.message, "wormhole_requires_http2");
}
