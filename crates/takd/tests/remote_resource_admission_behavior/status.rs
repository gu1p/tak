use std::thread;
use std::time::{Duration, Instant};

use prost::Message;
use tak_proto::{ContainerResourceLimits, NodeStatusResponse, PollTaskEventsResponse};
use takd::{RemoteNodeContext, SubmitAttemptStore, handle_remote_v1_request};

const REMOTE_ADMISSION_WAIT_TIMEOUT: Duration = Duration::from_secs(30);

pub(super) fn majority_memory_limits(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
) -> ContainerResourceLimits {
    let _ = status(context, store);
    let status = status(context, store);
    let memory = status.memory.expect("memory status");
    ContainerResourceLimits {
        cpu_cores: 0.1,
        memory_mb: memory
            .tak_admission_available_bytes
            .map(|bytes| bytes / 1024 / 1024)
            .unwrap_or_else(|| memory.total_bytes / 1024 / 1024)
            .saturating_mul(51)
            / 100,
    }
}

pub(super) fn status(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
) -> NodeStatusResponse {
    let response = handle_remote_v1_request(context, store, "GET", "/v1/node/status", None)
        .expect("status response");
    NodeStatusResponse::decode(response.body.as_slice()).expect("decode node status")
}

pub(super) fn wait_for_status(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    predicate: impl Fn(&NodeStatusResponse) -> bool,
) -> NodeStatusResponse {
    let deadline = Instant::now() + REMOTE_ADMISSION_WAIT_TIMEOUT;
    loop {
        let status = status(context, store);
        if predicate(&status) {
            return status;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for expected node status: {status:?}"
        );
        thread::sleep(Duration::from_millis(20));
    }
}

pub(super) fn task_events(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    task_run_id: &str,
) -> Vec<tak_proto::RemoteEvent> {
    let response = handle_remote_v1_request(
        context,
        store,
        "GET",
        &format!("/v1/tasks/{task_run_id}/events"),
        None,
    )
    .expect("events response");
    PollTaskEventsResponse::decode(response.body.as_slice())
        .expect("decode events")
        .events
}

pub(super) fn wait_for_task_event(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    task_run_id: &str,
    kind: &str,
) -> Vec<tak_proto::RemoteEvent> {
    let deadline = Instant::now() + REMOTE_ADMISSION_WAIT_TIMEOUT;
    loop {
        let events = task_events(context, store, task_run_id);
        if events.iter().any(|event| event.kind == kind) {
            return events;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {kind} event for {task_run_id}: {events:?}"
        );
        thread::sleep(Duration::from_millis(20));
    }
}
