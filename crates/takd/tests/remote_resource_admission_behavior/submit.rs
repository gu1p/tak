use prost::Message;
use tak_proto::{
    CmdStep, ContainerResourceLimits, ContainerRuntime, RuntimeSpec, Step, SubmitTaskRequest,
    runtime_spec, step,
};
use takd::{RemoteNodeContext, SubmitAttemptStore, handle_remote_v1_request};

use crate::support::remote_output::empty_workspace_zip;

pub(super) fn submit(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    task_run_id: &str,
    command: &str,
    limits: ContainerResourceLimits,
) {
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
        runtime: Some(RuntimeSpec {
            kind: Some(runtime_spec::Kind::Container(ContainerRuntime {
                image: Some("alpine:3.20".into()),
                dockerfile: None,
                build_context: None,
                resource_limits: Some(limits),
            })),
        }),
        task_label: "//apps/web:build".to_string(),
        needs: Vec::new(),
        outputs: Vec::new(),
        session: None,
        origin: Some("task".into()),
        runtime_source: Some("image:alpine:3.20".into()),
        command: Some(format!("sh -c '{}'", command.replace('\'', "'\\''"))),
        fused_members: Vec::new(),
        execution_label: None,
    };
    let response = handle_remote_v1_request(
        context,
        store,
        "POST",
        "/v1/tasks/submit",
        Some(&submit.encode_to_vec()),
    )
    .expect("submit response");
    assert_eq!(response.status_code, 200);
    let submit = tak_proto::SubmitTaskResponse::decode(response.body.as_slice())
        .expect("decode submit response");
    assert!(submit.accepted, "submit should be accepted: {submit:?}");
}
