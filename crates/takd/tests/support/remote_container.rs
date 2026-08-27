use super::remote_output::empty_workspace_zip;
use prost::Message;
use tak_proto::{
    ContainerResourceLimits, ContainerRuntime, FusedTaskMember, RuntimeSpec, SubmitTaskRequest,
    SubmitTaskResponse, runtime_spec,
};
use takd::{RemoteNodeContext, SubmitAttemptStore};

mod command;
mod fused;
mod result;
mod runtime_config;
use command::command_step;
pub use fused::submit_fused_container_task_with_retry;
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
    submit_container_task_with_members(
        context,
        store,
        task_run_id,
        command,
        resource_limits,
        Vec::new(),
    )
}

pub(super) fn submit_container_task_with_members(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    task_run_id: &str,
    command: &str,
    resource_limits: ContainerResourceLimits,
    fused_members: Vec<FusedTaskMember>,
) -> SubmitTaskResponse {
    let submit = SubmitTaskRequest {
        task_run_id: task_run_id.to_string(),
        attempt: 1,
        workspace_zip: empty_workspace_zip(),
        steps: vec![command_step(command)],
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
        fused_members,
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
