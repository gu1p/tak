use super::remote_output::empty_workspace_zip;
use crate::support::env::EnvGuard;
use crate::support::fake_docker::install_fake_docker;
use prost::Message;
use std::{env, path::Path};
use tak_proto::{
    CmdStep, ContainerResourceLimits, ContainerRuntime, RuntimeSpec, Step, SubmitTaskRequest,
    SubmitTaskResponse, runtime_spec, step,
};
use takd::{RemoteNodeContext, RemoteRuntimeConfig, SubmitAttemptStore, handle_remote_v1_request};

mod result;
pub use result::fetch_result;

pub fn configure_fake_docker_env(
    root: &Path,
    socket_path: &Path,
    env_guard: &mut EnvGuard,
) -> RemoteRuntimeConfig {
    let bin_root = root.join("bin");
    install_fake_docker(&bin_root);
    env_guard.set(
        "PATH",
        format!(
            "{}:{}",
            bin_root.display(),
            env::var("PATH").unwrap_or_default()
        ),
    );
    let docker_host = format!("unix://{}", socket_path.display());
    env_guard.set("DOCKER_HOST", &docker_host);
    RemoteRuntimeConfig::for_tests().with_docker_host(docker_host)
}

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
