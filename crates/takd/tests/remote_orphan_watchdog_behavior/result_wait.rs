use std::time::{Duration, Instant};

use prost::Message;
use tak_proto::GetTaskResultResponse;
use takd::{RemoteNodeContext, SubmitAttemptStore};

pub(super) async fn wait_for_result(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    task: &str,
) -> GetTaskResultResponse {
    let path = format!("/v1/tasks/{task}/result?attempt=1");
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let response =
            takd::daemon::remote::handle_remote_v1_request(context, store, "GET", &path, &[], None)
                .expect("result response");
        match response.status_code {
            200 => {
                return GetTaskResultResponse::decode(response.body.as_slice())
                    .expect("decode result");
            }
            404 if Instant::now() < deadline => {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            status => panic!("terminal result was not persisted: HTTP {status}"),
        }
    }
}
