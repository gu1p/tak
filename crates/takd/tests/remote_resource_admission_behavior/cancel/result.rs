use std::time::{Duration, Instant};

use prost::Message;
use tak_proto::GetTaskResultResponse;
use takd::{RemoteNodeContext, SubmitAttemptStore, handle_remote_v1_request};

pub(super) fn wait_for_result(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    task_run_id: &str,
) -> GetTaskResultResponse {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let response = handle_remote_v1_request(
            context,
            store,
            "GET",
            &format!("/v1/tasks/{task_run_id}/result?attempt=1"),
            None,
        )
        .expect("result response");
        if response.status_code == 200 {
            return GetTaskResultResponse::decode(response.body.as_slice()).expect("decode result");
        }
        assert!(Instant::now() < deadline, "timed out waiting for result");
        std::thread::sleep(Duration::from_millis(20));
    }
}
