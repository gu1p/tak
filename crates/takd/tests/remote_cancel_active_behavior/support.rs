use std::time::Duration;

use prost::Message;
use tak_proto::{GetTaskResultResponse, NodeStatusResponse as NodeStatus};
use takd::{RemoteNodeContext, SubmitAttemptStore};

use crate::support::fake_docker_daemon::FakeDockerDaemon;

pub(super) async fn wait_until_container_created(daemon: &FakeDockerDaemon) {
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while daemon.create_records().is_empty() {
        assert!(std::time::Instant::now() < deadline, "condition timed out");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

pub(super) async fn wait_for_result(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    task_run_id: &str,
) -> GetTaskResultResponse {
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    loop {
        let response = takd::daemon::remote::handle_remote_v1_request(
            context,
            store,
            "GET",
            &format!("/v1/tasks/{task_run_id}/result?attempt=1"),
            &[],
            None,
        )
        .expect("result response");
        if response.status_code == 200 {
            return GetTaskResultResponse::decode(response.body.as_slice()).expect("decode result");
        }
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting for result"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

pub(super) fn node_status(context: &RemoteNodeContext, store: &SubmitAttemptStore) -> NodeStatus {
    let response = takd::daemon::remote::handle_remote_v1_request(
        context,
        store,
        "GET",
        "/v1/node/status",
        &[],
        None,
    )
    .expect("status response");
    NodeStatus::decode(response.body.as_slice()).expect("decode status")
}
