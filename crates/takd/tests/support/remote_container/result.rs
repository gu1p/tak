use prost::Message;
use tak_proto::GetTaskResultResponse;
use takd::{RemoteNodeContext, SubmitAttemptStore};

pub fn fetch_result(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    task_run_id: &str,
) -> GetTaskResultResponse {
    let response = takd::daemon::remote::handle_remote_v1_request(
        context,
        store,
        "GET",
        &format!("/v1/tasks/{task_run_id}/result"),
        &[],
        None,
    )
    .expect("result response");
    GetTaskResultResponse::decode(response.body.as_slice()).expect("decode result")
}
