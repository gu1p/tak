use prost::Message;
use tak_proto::{CmdStep, Step, SubmitTaskRequest, SubmitTaskResponse, step};
use takd::{RemoteNodeContext, SubmitAttemptStore, handle_remote_v1_request};

pub fn test_context() -> RemoteNodeContext {
    RemoteNodeContext::new(
        tak_proto::NodeInfo {
            node_id: "builder-a".into(),
            display_name: "builder-a".into(),
            base_url: "http://127.0.0.1:43123".into(),
            healthy: true,
            pools: vec!["default".into()],
            tags: vec!["builder".into()],
            capabilities: vec!["linux".into()],
            transport: "direct".into(),
        },
        "secret".into(),
    )
}

pub fn submit_shell_task(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    task_run_id: &str,
    command: &str,
) -> SubmitTaskResponse {
    let submit = SubmitTaskRequest {
        task_run_id: task_run_id.to_string(),
        attempt: 1,
        workspace_zip: empty_workspace_zip(),
        steps: vec![Step {
            kind: Some(step::Kind::Cmd(CmdStep {
                argv: vec!["sh".to_string(), "-c".to_string(), command.to_string()],
                cwd: None,
                env: Default::default(),
            })),
        }],
        timeout_s: None,
        runtime: None,
        task_label: "//apps/web:test".to_string(),
        needs: Vec::new(),
    };
    let submit = handle_remote_v1_request(
        context,
        store,
        "POST",
        "/v1/tasks/submit",
        Some(&submit.encode_to_vec()),
    )
    .expect("submit response");
    SubmitTaskResponse::decode(submit.body.as_slice()).expect("decode submit")
}

pub fn empty_workspace_zip() -> Vec<u8> {
    let cursor = std::io::Cursor::new(Vec::new());
    let writer = zip::ZipWriter::new(cursor);
    writer
        .finish()
        .expect("finish empty workspace zip")
        .into_inner()
}
