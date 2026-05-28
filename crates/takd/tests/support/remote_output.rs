#![allow(dead_code)]

use tak_proto::{
    ContainerResourceLimits, ContainerRuntime, OutputSelector, RuntimeSpec, SubmitTaskResponse,
    runtime_spec,
};
use takd::{RemoteNodeContext, RemoteRuntimeConfig, SubmitAttemptStore};

#[path = "remote_output/submit.rs"]
mod submit;

use submit::submit_shell_task_with_outputs_and_runtime;

pub fn test_context() -> RemoteNodeContext {
    test_context_with_runtime(RemoteRuntimeConfig::for_tests())
}

pub fn test_context_with_runtime(runtime_config: RemoteRuntimeConfig) -> RemoteNodeContext {
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
            transport_state: "ready".into(),
            transport_detail: String::new(),
        },
        "secret".into(),
        runtime_config,
    )
}

pub fn submit_shell_task(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    task_run_id: &str,
    command: &str,
) -> SubmitTaskResponse {
    submit_shell_task_with_outputs(context, store, task_run_id, command, Vec::new())
}

pub fn submit_shell_task_with_outputs(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    task_run_id: &str,
    command: &str,
    outputs: Vec<OutputSelector>,
) -> SubmitTaskResponse {
    submit_shell_task_with_outputs_and_runtime(
        context,
        store,
        task_run_id,
        command,
        outputs,
        test_container_runtime(),
    )
}

pub fn submit_shell_task_with_limits(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    task_run_id: &str,
    command: &str,
    resource_limits: ContainerResourceLimits,
) -> SubmitTaskResponse {
    submit_shell_task_with_outputs_and_runtime(
        context,
        store,
        task_run_id,
        command,
        Vec::new(),
        test_container_runtime_with_limits(resource_limits),
    )
}

pub fn empty_workspace_zip() -> Vec<u8> {
    let cursor = std::io::Cursor::new(Vec::new());
    let writer = zip::ZipWriter::new(cursor);
    writer
        .finish()
        .expect("finish empty workspace zip")
        .into_inner()
}

include!("remote_output/runtime.rs");
