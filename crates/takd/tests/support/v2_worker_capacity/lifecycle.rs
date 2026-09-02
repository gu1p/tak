use std::time::Duration;

use tak_proto::worker_v2::{
    CancelAttemptRequest, DispatchAttemptRequest, ObserveAttemptRequest, WorkerAttemptState,
    decode_observe_response, encode_cancel_request, encode_observe_request,
};

use super::{RunningServer, post, snapshot, status};

pub async fn cancel(server: &RunningServer, request: &DispatchAttemptRequest) {
    let value = CancelAttemptRequest {
        protocol_version: 2,
        identity: request.identity.clone(),
    };
    let response = post(
        server,
        "/v2/attempts/cancel",
        Some("secret"),
        &["v2"],
        &encode_cancel_request(&value).unwrap(),
    )
    .await;
    assert!(matches!(status(&response), 200 | 202));
}

pub async fn wait_terminal(server: &RunningServer, request: &DispatchAttemptRequest) {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let value = ObserveAttemptRequest {
                protocol_version: 2,
                identity: request.identity.clone(),
                after_event: 0,
            };
            let response = post(
                server,
                "/v2/attempts/observe",
                Some("secret"),
                &["v2"],
                &encode_observe_request(&value).unwrap(),
            )
            .await;
            if decode_observe_response(&response.body, &request.identity.fencing_token)
                .unwrap()
                .state
                == WorkerAttemptState::Completed
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
}

pub async fn wait_released(server: &RunningServer) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while snapshot(server).await.usage.execution_slots != 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}
