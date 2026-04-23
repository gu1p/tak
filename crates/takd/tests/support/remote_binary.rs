#![allow(dead_code)]

use prost::Message;
use std::thread;
use std::time::Duration;
use tak_proto::{
    CmdStep, ContainerRuntime, NodeInfo, PollTaskEventsResponse, RuntimeSpec, Step,
    SubmitTaskRequest, runtime_spec, step,
};
use takd::{RemoteNodeContext, RemoteRuntimeConfig, SubmitAttemptStore, handle_remote_v1_request};

pub fn streaming_context() -> RemoteNodeContext {
    RemoteNodeContext::new(
        NodeInfo {
            node_id: "builder-stream".into(),
            display_name: "builder-stream".into(),
            base_url: "http://127.0.0.1:43123".into(),
            healthy: true,
            pools: vec!["default".into()],
            tags: vec!["builder".into()],
            capabilities: vec!["linux".into()],
            transport: "direct".into(),
            transport_state: "ready".into(),
            transport_detail: String::new(),
        },
        "secret".into(),
        RemoteRuntimeConfig::for_tests().with_skip_exec_root_probe(true),
    )
}

pub fn streaming_submit_request() -> SubmitTaskRequest {
    streaming_submit_request_with_command(
        "task-run-stream",
        "printf 'remote-stdout\\n'; printf 'remote-stderr\\n' >&2",
    )
}

pub fn streaming_submit_request_with_command(
    task_run_id: &str,
    command: &str,
) -> SubmitTaskRequest {
    SubmitTaskRequest {
        task_run_id: task_run_id.to_string(),
        attempt: 1,
        workspace_zip: empty_workspace_zip(),
        steps: vec![Step {
            kind: Some(step::Kind::Cmd(CmdStep {
                argv: vec!["sh".into(), "-c".into(), command.into()],
                cwd: None,
                env: Default::default(),
            })),
        }],
        timeout_s: None,
        runtime: Some(test_container_runtime()),
        task_label: "//apps/web:stream".to_string(),
        needs: Vec::new(),
        outputs: Vec::new(),
    }
}
pub fn wait_for_streaming_events(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
) -> PollTaskEventsResponse {
    wait_for_streaming_events_for_task(context, store, "task-run-stream")
}

pub fn wait_for_streaming_events_for_task(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    task_run_id: &str,
) -> PollTaskEventsResponse {
    loop {
        let path = format!("/v1/tasks/{task_run_id}/events");
        let response =
            handle_remote_v1_request(context, store, "GET", &path, None).expect("events response");
        let events =
            PollTaskEventsResponse::decode(response.body.as_slice()).expect("decode events");
        if events.done {
            return events;
        }
        thread::sleep(Duration::from_millis(20));
    }
}

fn empty_workspace_zip() -> Vec<u8> {
    zip::ZipWriter::new(std::io::Cursor::new(Vec::new()))
        .finish()
        .expect("finish empty workspace zip")
        .into_inner()
}

fn test_container_runtime() -> RuntimeSpec {
    RuntimeSpec {
        kind: Some(runtime_spec::Kind::Container(ContainerRuntime {
            image: Some("alpine:3.20".into()),
            dockerfile: None,
            build_context: None,
        })),
    }
}
