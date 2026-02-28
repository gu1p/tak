/// Runs one task with retries, acquiring and releasing leases per attempt when configured.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
async fn run_single_task(
    task: &ResolvedTask,
    workspace_root: &Path,
    options: &RunOptions,
    lease_context: &LeaseContext,
) -> Result<TaskRunResult> {
    let mut placement = preflight_task_placement(task, workspace_root).await?;
    let mut runtime_metadata = resolve_initial_runtime_metadata(task, &mut placement).await?;
    let remote_workspace = if placement.placement_mode == PlacementMode::Remote {
        Some(stage_remote_workspace(task, workspace_root)?)
    } else {
        None
    };
    let run_root = remote_workspace
        .as_ref()
        .map(|staged| staged.temp_dir.path().to_path_buf())
        .unwrap_or_else(|| workspace_root.to_path_buf());

    let total_attempts = task.retry.attempts.max(1);
    let mut attempt = 0;
    let task_run_id = Uuid::new_v4().to_string();
    let task_label = task.label.to_string();

    loop {
        attempt += 1;

        let submit = resolve_attempt_submit_state(
            task,
            &mut placement,
            &mut runtime_metadata,
            remote_workspace.as_ref(),
            &task_run_id,
            &task_label,
            attempt,
        )
        .await?;

        let attempt_context = AttemptExecutionContext {
            task,
            workspace_root,
            run_root: &run_root,
            placement: &placement,
            runtime_metadata: runtime_metadata.as_ref(),
            remote_workspace: remote_workspace.as_ref(),
            task_run_id: &task_run_id,
            attempt,
        };
        let lease_id = acquire_task_lease(task, attempt, options, lease_context).await?;
        let attempt_result = execute_task_attempt(&attempt_context, submit).await;

        if let Some(id) = lease_id.as_ref() {
            release_task_lease(id, options)
                .await
                .context(format!("failed releasing lease for {}", task.label))?;
        }

        let outcome = attempt_result?;
        if outcome.attempt_success {
            return Ok(build_task_run_result(
                attempt,
                true,
                &placement,
                remote_workspace.as_ref(),
                runtime_metadata.as_ref(),
                outcome,
            ));
        }

        let can_retry =
            attempt < total_attempts && should_retry(outcome.last_exit_code, &task.retry.on_exit);
        if !can_retry {
            return Ok(build_task_run_result(
                attempt,
                false,
                &placement,
                remote_workspace.as_ref(),
                runtime_metadata.as_ref(),
                outcome,
            ));
        }

        let wait = retry_backoff_delay(&task.retry.backoff, attempt);
        if !wait.is_zero() {
            tokio::time::sleep(wait).await;
        }
    }
}
