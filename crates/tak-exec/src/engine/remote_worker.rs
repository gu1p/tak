pub async fn execute_remote_worker_steps(
    workspace_root: &Path,
    spec: &RemoteWorkerExecutionSpec,
) -> Result<RemoteWorkerExecutionResult> {
    let task = ResolvedTask {
        label: TaskLabel {
            package: "//".to_string(),
            name: "remote_worker_task".to_string(),
        },
        doc: String::new(),
        deps: Vec::new(),
        steps: spec.steps.clone(),
        needs: Vec::new(),
        queue: None,
        retry: RetryDef::default(),
        timeout_s: spec.timeout_s,
        context: tak_core::model::CurrentStateSpec::default(),
        execution: TaskExecutionSpec::LocalOnly(tak_core::model::LocalSpec::default()),
        tags: Vec::new(),
    };

    let runtime_metadata = match spec.runtime.as_ref() {
        Some(runtime) => resolve_runtime_execution_metadata_for_node_runtime(
            &task,
            &spec.node_id,
            Some(runtime),
        )?,
        None => None,
    };

    let result =
        run_task_steps_with_runtime(&task, workspace_root, runtime_metadata.as_ref()).await?;
    Ok(RemoteWorkerExecutionResult {
        success: result.success,
        exit_code: result.exit_code,
        runtime_kind: runtime_metadata
            .as_ref()
            .map(|metadata| metadata.kind.clone()),
        runtime_engine: runtime_metadata
            .as_ref()
            .and_then(|metadata| metadata.engine.clone()),
    })
}
