use std::collections::BTreeMap;
use std::time::Duration;

use tak_core::v2::Step;
use tak_proto::worker_v2::{
    ObserveAttemptRequest, WorkerAttemptState, WorkerTerminalOutcome, decode_observe_response,
    encode_dispatch_request, encode_observe_request, payload_digest,
};

use crate::support::{
    sqlite_gate,
    v2_worker_cache::ensure,
    v2_worker_execution::{output_archive, output_dispatch},
    v2_worker_http::{post, status},
    worker_http::start_server,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn transient_output_store_lock_does_not_replace_a_successful_child_result() {
    let server = start_server().await;
    let child_ready = server.state_root.join("child-ready");
    let release_child = server.state_root.join("release-child");
    let mut request = output_dispatch();
    request.payload.tasks[0].steps = vec![Step::Cmd {
        argv: vec![
            "/bin/sh".into(),
            "-c".into(),
            "printf ready > \"$CHILD_READY\"; while [ ! -e \"$RELEASE_CHILD\" ]; do sleep 0.01; done; printf 'child succeeded\\n'".into(),
        ],
        cwd: None,
        env: BTreeMap::from([
            ("CHILD_READY".into(), child_ready.display().to_string()),
            ("RELEASE_CHILD".into(), release_child.display().to_string()),
        ]),
    }];
    request.payload.tasks[0].outputs.clear();
    request.payload_digest = payload_digest(&request.payload).unwrap();
    ensure(
        &server,
        &request.payload.workspace.descriptor,
        &output_archive(),
    )
    .await;

    let response = post(
        &server,
        "/v2/attempts/dispatch",
        Some("secret"),
        &["v2"],
        &encode_dispatch_request(&request).unwrap(),
    )
    .await;
    assert_eq!(status(&response), 202);
    tokio::time::timeout(Duration::from_secs(3), async {
        while !child_ready.exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("child should reach its output gate");
    let gate = sqlite_gate::begin_immediate(&server.state_root.join("takd.sqlite"));
    std::fs::write(release_child, b"").unwrap();
    std::thread::sleep(Duration::from_millis(5_250));
    drop(gate);

    let observed = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let response = post(
                &server,
                "/v2/attempts/observe",
                Some("secret"),
                &["v2"],
                &encode_observe_request(&ObserveAttemptRequest {
                    protocol_version: 2,
                    identity: request.identity.clone(),
                    after_event: 0,
                })
                .unwrap(),
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
    .expect("worker attempt should finish after the transient lock clears");

    let terminal = observed.terminal.unwrap();
    assert_eq!(terminal.outcome, WorkerTerminalOutcome::Succeeded);
    assert_eq!(terminal.exit_code, Some(0));
    assert_eq!(observed.events.len(), 1);
}
