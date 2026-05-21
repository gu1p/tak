use std::thread;
use std::time::Duration;

use prost::Message;
use tak_proto::{ContainerResourceLimits, NodeStatusResponse, PollTaskEventsResponse};
use takd::{RemoteNodeContext, SubmitAttemptStore, handle_remote_v1_request};

pub(super) fn full_node_limits(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
) -> ContainerResourceLimits {
    let status = status(context, store);
    let cpu = status.cpu.expect("cpu status");
    let memory = status.memory.expect("memory status");
    ContainerResourceLimits {
        cpu_cores: f64::from(cpu.logical_cores.max(1)),
        memory_mb: (memory.total_bytes / 1024 / 1024).max(1),
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
    let mut last = None;
    for _ in 0..100 {
        let status = status(context, store);
        if predicate(&status) {
            return status;
        }
        last = Some(status);
        thread::sleep(Duration::from_millis(20));
    }
    panic!("timed out waiting for expected node status: {last:?}");
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
    for _ in 0..100 {
        let events = task_events(context, store, task_run_id);
        if events.iter().any(|event| event.kind == kind) {
            return events;
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("timed out waiting for {kind} event for {task_run_id}");
}
