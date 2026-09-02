use std::num::NonZeroU32;
use tak_core::v2::{ResourceRequest, Step};
use tak_proto::worker_v2::{
    DispatchAttemptRequest, WorkerSnapshot, decode_snapshot, encode_dispatch_request,
    payload_digest,
};

use super::v2_worker_cache::ensure;
use super::v2_worker_execution::{output_archive, output_dispatch};
use super::v2_worker_http::{get, post, status};
use super::worker_http::RunningServer;

mod lifecycle;
pub use lifecycle::{cancel, wait_released, wait_terminal};

pub fn request(
    identity: (&str, &str, &str),
    resources: (u64, u64, u32),
    command: &str,
) -> DispatchAttemptRequest {
    let mut request = output_dispatch();
    request.identity.run_id = identity.0.into();
    request.identity.job_id = identity.1.into();
    request.identity.node_id = "builder-a".into();
    request.identity.fencing_token = identity.2.into();
    request.payload.tasks[0].job_id = identity.1.into();
    request.payload.tasks[0].steps = vec![Step::Cmd {
        argv: vec!["/bin/sh".into(), "-c".into(), command.into()],
        cwd: None,
        env: Default::default(),
    }];
    request.payload.tasks[0].outputs.clear();
    request.payload.resources = ResourceRequest {
        cpu_millis: resources.0,
        memory_bytes: resources.1,
        execution_slots: NonZeroU32::new(resources.2).unwrap(),
    };
    request.payload_digest = payload_digest(&request.payload).unwrap();
    request
}

pub async fn snapshot(server: &RunningServer) -> WorkerSnapshot {
    let response = get(server, "/v2/worker/snapshot", Some("secret"), &["v2"]).await;
    assert_eq!(status(&response), 200);
    decode_snapshot(&response.body).unwrap()
}

pub async fn dispatch(server: &RunningServer, request: &DispatchAttemptRequest) -> u16 {
    ensure(
        server,
        &request.payload.workspace.descriptor,
        &output_archive(),
    )
    .await;
    let response = post(
        server,
        "/v2/attempts/dispatch",
        Some("secret"),
        &["v2"],
        &encode_dispatch_request(request).unwrap(),
    )
    .await;
    status(&response)
}
