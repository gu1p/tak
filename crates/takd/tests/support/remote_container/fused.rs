use tak_proto::{ContainerResourceLimits, FusedTaskMember, RetryPolicy, SubmitTaskResponse};
use takd::{RemoteNodeContext, SubmitAttemptStore};

use super::{command_step, submit_container_task_with_members};

pub fn submit_fused_container_task_with_retry(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    task_run_id: &str,
    command: &str,
    attempts: u32,
) -> SubmitTaskResponse {
    let member = FusedTaskMember {
        task_label: "//apps/web:fused".to_string(),
        steps: vec![command_step(command)],
        timeout_s: None,
        retry: Some(RetryPolicy {
            attempts,
            on_exit: vec![137],
            backoff: None,
        }),
        execution_label: Some("test.fused".to_string()),
    };
    submit_container_task_with_members(
        context,
        store,
        task_run_id,
        command,
        ContainerResourceLimits {
            cpu_cores: 1.0,
            memory_mb: 512,
        },
        vec![member],
    )
}
