use super::remote_output::empty_workspace_zip;
use prost::Message;
use tak_proto::{
    CmdStep, ContainerResourceLimits, ContainerRuntime, RuntimeSpec, Step, SubmitTaskRequest,
    SubmitTaskResponse, runtime_spec, step,
};
use takd::{RemoteNodeContext, SubmitAttemptStore};

mod result;
mod runtime_config;
pub use result::fetch_result;
pub use runtime_config::configure_fake_docker_env;

pub fn submit_container_task(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    task_run_id: &str,
    command: &str,
) -> SubmitTaskResponse {
    submit_container_task_with_limits(
        context,
        store,
        task_run_id,
        command,
        ContainerResourceLimits {
            cpu_cores: 1.0,
            memory_mb: 512,
        },
    )
}

pub fn submit_container_task_with_limits(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    task_run_id: &str,
    command: &str,
    resource_limits: ContainerResourceLimits,
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
        runtime: Some(RuntimeSpec {
            kind: Some(runtime_spec::Kind::Container(ContainerRuntime {
                image: Some("alpine:3.20".to_string()),
                dockerfile: None,
                build_context: None,
                resource_limits: Some(resource_limits),
            })),
        }),
        task_label: "//apps/web:test".to_string(),
        needs: Vec::new(),
        outputs: Vec::new(),
        session: None,
        origin: Some("task".into()),
        runtime_source: Some("image:alpine:3.20".into()),
        command: Some(format!("sh -c '{}'", command.replace('\'', "'\\''"))),
        fused_members: Vec::new(),
        execution_label: None,
        workspace_upload: None,
    };
    let submit = takd::daemon::remote::handle_remote_v1_request(
        context,
        store,
        "POST",
        "/v1/tasks/submit",
        &[],
        Some(&submit.encode_to_vec()),
    )
    .expect("submit response");
    SubmitTaskResponse::decode(submit.body.as_slice()).expect("decode submit")
}
