use crate::support;

use std::thread;
use std::time::Duration;

use prost::Message;
use tak_proto::{CmdStep, NodeStatusResponse, Step, SubmitTaskRequest, step};
use takd::{RemoteRuntimeConfig, SubmitAttemptStore, handle_remote_v1_request};

use support::remote_output::{
    empty_workspace_zip, test_container_runtime, test_context_with_runtime,
};

#[test]
fn remote_status_route_serves_protobuf_and_reports_running_job() {
    let _env_lock = support::env::env_lock();
    let mut env = support::env::EnvGuard::default();
    env.set("TAK_TEST_HOST_PLATFORM", "other");
    let context =
        test_context_with_runtime(RemoteRuntimeConfig::for_tests().with_skip_exec_root_probe(true));
    let temp = tempfile::tempdir().expect("tempdir");
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let submit = SubmitTaskRequest {
        task_run_id: "task-run-1".to_string(),
        attempt: 1,
        workspace_zip: empty_workspace_zip(),
        steps: vec![Step {
            kind: Some(step::Kind::Cmd(CmdStep {
                argv: vec!["sh".to_string(), "-c".to_string(), "sleep 1".to_string()],
                cwd: None,
                env: Default::default(),
            })),
        }],
        timeout_s: None,
        runtime: Some(test_container_runtime()),
        task_label: "//apps/web:build".to_string(),
        needs: vec![tak_proto::SubmittedNeed {
            name: "cpu".to_string(),
            scope: "machine".to_string(),
            scope_key: None,
            slots: 2.0,
        }],
        outputs: Vec::new(),
        session: None,
    };
    let submit = handle_remote_v1_request(
        &context,
        &store,
        "POST",
        "/v1/tasks/submit",
        Some(&submit.encode_to_vec()),
    )
    .expect("submit response");
    assert_eq!(submit.status_code, 200);
    for _ in 0..50 {
        let response = handle_remote_v1_request(&context, &store, "GET", "/v1/node/status", None)
            .expect("status response");
        assert_eq!(response.content_type, "application/x-protobuf");
        let status =
            NodeStatusResponse::decode(response.body.as_slice()).expect("decode node status");
        if !status.active_jobs.is_empty() {
            assert_eq!(status.node.expect("node").node_id, "builder-a");
            assert_eq!(status.active_jobs[0].task_label, "//apps/web:build");
            assert_eq!(status.active_jobs[0].attempt, 1);
            assert_eq!(status.active_jobs[0].needs.len(), 1);
            return;
        }
        thread::sleep(Duration::from_millis(20));
    }

    panic!("timed out waiting for active job in node status");
}
