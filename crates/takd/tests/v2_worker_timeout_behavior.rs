use std::collections::BTreeMap;
use std::time::Duration;

use tak_core::v2::Step;
use tak_proto::worker_v2::{
    ObserveAttemptRequest, WorkerAttemptState, WorkerTerminalOutcome, decode_observe_response,
    encode_dispatch_request, encode_observe_request, payload_digest,
};

use crate::support::{
    worker_http::start_server,
    v2_worker_cache::ensure,
    v2_worker_execution::{output_archive, output_dispatch},
    v2_worker_http::{post, status},
};

#[tokio::test]
async fn worker_v2_enforces_the_resolved_task_timeout() {
    let server = start_server().await;
    let mut request = output_dispatch();
    request.payload.tasks[0].steps = vec![Step::Cmd {
        argv: vec!["/bin/sh".into(), "-c".into(), "sleep 2".into()],
        cwd: None,
        env: BTreeMap::new(),
    }];
    request.payload.tasks[0].outputs.clear();
    request.payload.tasks[0].timeout_s = Some(1);
    request.payload_digest = payload_digest(&request.payload).unwrap();
    ensure(
        &server,
        &request.payload.workspace.descriptor,
        &output_archive(),
    )
    .await;
    let accepted = post(
        &server,
        "/v2/attempts/dispatch",
        Some("secret"),
        &["v2"],
        &encode_dispatch_request(&request).unwrap(),
    )
    .await;
    assert_eq!(status(&accepted), 202);

    let observed = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let observe = ObserveAttemptRequest {
                protocol_version: 2,
                identity: request.identity.clone(),
                after_event: 0,
            };
            let response = post(
                &server,
                "/v2/attempts/observe",
                Some("secret"),
                &["v2"],
                &encode_observe_request(&observe).unwrap(),
            )
            .await;
            let observed = decode_observe_response(&response.body, "fence-1").unwrap();
            if observed.state == WorkerAttemptState::Completed {
                break observed;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        observed.terminal.unwrap().outcome,
        WorkerTerminalOutcome::Failed
    );
}
