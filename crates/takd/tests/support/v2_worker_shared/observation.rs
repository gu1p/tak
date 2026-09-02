use std::time::Duration;

use tak_proto::worker_v2::{
    DispatchAttemptRequest, ObserveAttemptRequest, ObserveAttemptResponse, WorkerAttemptState,
    decode_observe_response, encode_observe_request,
};

use super::{RunningServer, post};

pub async fn wait_terminal(
    server: &RunningServer,
    request: &DispatchAttemptRequest,
) -> ObserveAttemptResponse {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let query = ObserveAttemptRequest {
                protocol_version: 2,
                identity: request.identity.clone(),
                after_event: 0,
            };
            let response = post(
                server,
                "/v2/attempts/observe",
                Some("secret"),
                &["v2"],
                &encode_observe_request(&query).unwrap(),
            )
            .await;
            let observed =
                decode_observe_response(&response.body, &request.identity.fencing_token).unwrap();
            if observed.state == WorkerAttemptState::Completed {
                return observed;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap()
}
