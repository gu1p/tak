use std::thread;
use std::time::Duration;

use prost::Message;
use tak_proto::{CmdStep, PollTaskEventsResponse, Step, SubmitTaskRequest, SubmittedNeed, step};
use takd::{RemoteNodeContext, SubmitAttemptStore};

use crate::support::remote_output::{empty_workspace_zip, test_container_runtime};

pub(super) fn submit_request() -> SubmitTaskRequest {
    SubmitTaskRequest {
        task_run_id: "task-run-1".to_string(),
        attempt: 1,
        workspace_zip: empty_workspace_zip(),
        steps: vec![Step {
            kind: Some(step::Kind::Cmd(CmdStep {
                argv: vec!["sh".to_string(), "-c".to_string(), "true".to_string()],
                cwd: None,
                env: Default::default(),
            })),
        }],
        timeout_s: None,
        runtime: Some(test_container_runtime()),
        task_label: "//apps/web:test".to_string(),
        needs: vec![SubmittedNeed {
            name: "cpu".to_string(),
            scope: "machine".to_string(),
            scope_key: None,
            slots: 1.0,
        }],
        outputs: Vec::new(),
        session: None,
        origin: Some("task".into()),
        runtime_source: Some("image:alpine:3.20".into()),
        command: Some("sh -c true".into()),
        fused_members: Vec::new(),
        execution_label: None,
        workspace_upload: None,
    }
}

pub(super) fn wait_for_terminal_events(context: &RemoteNodeContext, store: &SubmitAttemptStore) {
    for _ in 0..50 {
        let events = takd::daemon::remote::handle_remote_v1_request(
            context,
            store,
            "GET",
            "/v1/tasks/task-run-1/events",
            &[],
            None,
        )
        .expect("events response");
        let events = PollTaskEventsResponse::decode(events.body.as_slice()).expect("decode events");
        if events.done {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
}
