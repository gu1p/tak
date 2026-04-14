use std::fs;
use std::os::unix::fs::symlink;

use prost::Message;
use tak_proto::{CmdStep, ErrorResponse, NodeInfo, Step, SubmitTaskRequest, step};
use takd::{
    RemoteNodeContext, SubmitAttemptStore, build_submit_idempotency_key, handle_remote_v1_request,
};

#[test]
fn remote_status_route_returns_status_unavailable_when_active_job_root_is_unreadable() {
    let temp = tempfile::tempdir().expect("tempdir");
    let exec_root_base = temp.path().join("exec-root");
    fs::create_dir_all(&exec_root_base).expect("create exec root");
    unsafe {
        std::env::set_var("TAKD_REMOTE_EXEC_ROOT", &exec_root_base);
    }
    let context = RemoteNodeContext::new(
        NodeInfo {
            node_id: "builder-a".into(),
            display_name: "builder-a".into(),
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
    );
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");

    let submit = SubmitTaskRequest {
        task_run_id: "task-run-1".to_string(),
        attempt: 1,
        workspace_zip: Vec::new(),
        steps: vec![Step {
            kind: Some(step::Kind::Cmd(CmdStep {
                argv: vec!["sh".into(), "-c".into(), "sleep 1".into()],
                cwd: None,
                env: Default::default(),
            })),
        }],
        timeout_s: None,
        runtime: None,
        task_label: "//apps/web:build".to_string(),
        needs: Vec::new(),
        outputs: Vec::new(),
    };
    let _ = handle_remote_v1_request(
        &context,
        &store,
        "POST",
        "/v1/tasks/submit",
        Some(&submit.encode_to_vec()),
    )
    .expect("submit response");

    let submit_key = build_submit_idempotency_key("task-run-1", Some(1)).expect("submit key");
    let execution_root = exec_root_base.join(submit_key.replace(':', "_"));
    fs::create_dir_all(execution_root.join("nested")).expect("create execution root");
    fs::remove_dir_all(&execution_root).expect("remove execution root");
    symlink(&execution_root, &execution_root).expect("self-referential execution root symlink");

    let response = handle_remote_v1_request(&context, &store, "GET", "/v1/node/status", None)
        .expect("status response");
    fs::remove_file(&execution_root).expect("remove execution root symlink");
    unsafe {
        std::env::remove_var("TAKD_REMOTE_EXEC_ROOT");
    }
    assert_eq!(response.status_code, 500);
    let error = ErrorResponse::decode(response.body.as_slice()).expect("decode error");
    assert_eq!(error.message, "status_unavailable");
}
