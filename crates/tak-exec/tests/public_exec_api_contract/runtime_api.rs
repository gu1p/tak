use tak_core::model::{RemoteRuntimeSpec, StepDef, TaskLabel};

#[test]
fn remote_worker_public_api_stays_available() {
    let _execute_remote_worker_steps = tak_exec::execute_remote_worker_steps;
    let _execute_remote_worker_steps_with_output =
        tak_exec::execute_remote_worker_steps_with_output;
    let task_label = TaskLabel {
        package: "//".to_string(),
        name: "task".to_string(),
    };

    let _remote_worker_spec = tak_exec::RemoteWorkerExecutionSpec {
        task_label,
        attempt: 1,
        steps: vec![StepDef::Cmd {
            argv: vec!["echo".to_string()],
            cwd: None,
            env: Default::default(),
        }],
        timeout_s: None,
        runtime: None::<RemoteRuntimeSpec>,
        node_id: "node".to_string(),
        container_user: None,
        image_cache: None,
        container_identity: Some(tak_exec::ContainerExecutionIdentity {
            owner: "takd".to_string(),
            submit_key: "task:1".to_string(),
            task_run_id: "task".to_string(),
        }),
    };
    let _remote_worker_result = tak_exec::RemoteWorkerExecutionResult {
        success: true,
        exit_code: Some(0),
        runtime_kind: None,
        runtime_engine: None,
    };
}
