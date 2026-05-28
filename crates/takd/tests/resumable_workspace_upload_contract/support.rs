use prost::Message;
use tak_proto::{
    BeginWorkspaceUploadRequest, BeginWorkspaceUploadResponse, FinishWorkspaceUploadResponse,
};
use takd::{RemoteNodeContext, SubmitAttemptStore, handle_remote_v1_request};

mod submit;
pub(super) use submit::submit_with_upload;

pub(super) fn post_begin(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    digest: &str,
    size_bytes: u64,
) -> BeginWorkspaceUploadResponse {
    let body = BeginWorkspaceUploadRequest {
        task_run_id: "run-1".into(),
        attempt: 1,
        sha256: digest.into(),
        size_bytes,
    }
    .encode_to_vec();
    let response = handle_remote_v1_request(
        context,
        store,
        "POST",
        "/v2/workspaces/uploads/begin",
        Some(&body),
    )
    .expect("begin");
    assert_eq!(response.status_code, 200);
    BeginWorkspaceUploadResponse::decode(response.body.as_slice()).expect("begin response")
}

pub(super) fn patch_chunk(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    upload_id: &str,
    offset: u64,
    chunk: &[u8],
) {
    let response = handle_remote_v1_request(
        context,
        store,
        "PATCH",
        &format!("/v2/workspaces/uploads/{upload_id}?offset={offset}"),
        Some(chunk),
    )
    .expect("append");
    assert_eq!(response.status_code, 200);
}

pub(super) fn post_finish(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    upload_id: &str,
) -> FinishWorkspaceUploadResponse {
    let response = handle_remote_v1_request(
        context,
        store,
        "POST",
        &format!("/v2/workspaces/uploads/{upload_id}/finish"),
        None,
    )
    .expect("finish");
    assert_eq!(response.status_code, 200);
    FinishWorkspaceUploadResponse::decode(response.body.as_slice()).expect("finish response")
}
