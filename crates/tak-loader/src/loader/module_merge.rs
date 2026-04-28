use std::path::Path;

use anyhow::{Result, anyhow, bail};
use tak_core::label::parse_label;
use tak_core::model::{LimiterKey, ModuleSpec, ResolvedTask, RetryDef};

use super::{
    MergeState,
    context_resolution::resolve_current_state,
    execution_policy_registry::resolve_task_execution,
    output_resolution::resolve_output_selectors,
    remote_validation::validate_runtime,
    scope_keys::{
        limiter_key_for_limiter, scope_key_for, scope_key_label, scope_label, with_scope_key,
    },
    session_resolution::register_reachable_sessions,
};

pub(crate) fn merge_module(
    module_path: &Path,
    root: &Path,
    project_id: &str,
    package: &str,
    module: ModuleSpec,
    state: &mut MergeState,
) -> Result<()> {
    register_reachable_sessions(
        module_path,
        package,
        module.defaults.execution.as_ref(),
        state,
    )?;
    for task in &module.tasks {
        register_reachable_sessions(module_path, package, task.execution.as_ref(), state)?;
    }

    for limiter in module.limiters {
        let key = limiter_key_for_limiter(&limiter, project_id, root);
        if let Some(previous) = state.limiter_origins.get(&key) {
            bail!(
                "duplicate limiter definition: {} (scope={} {})\nfirst defined in {}\nconflicts with {}",
                key.name,
                scope_label(&key.scope),
                scope_key_label(&key.scope_key),
                previous.display(),
                module_path.display()
            );
        }
        state
            .limiter_origins
            .insert(key.clone(), module_path.to_path_buf());
        state.limiters.insert(key, limiter);
    }

    for queue in module.queues {
        let key = LimiterKey {
            scope: queue.scope.clone(),
            scope_key: scope_key_for(&queue.scope, project_id, root),
            name: queue.name.clone(),
        };
        if let Some(previous) = state.queue_origins.get(&key) {
            bail!(
                "duplicate queue definition: {} (scope={} {})\nfirst defined in {}\nconflicts with {}",
                key.name,
                scope_label(&key.scope),
                scope_key_label(&key.scope_key),
                previous.display(),
                module_path.display()
            );
        }
        state
            .queue_origins
            .insert(key.clone(), module_path.to_path_buf());
        state.queues.insert(key, queue);
    }

    for task in module.tasks {
        let label = parse_label(&format!("{package}:{}", task.name), package)
            .map_err(|e| anyhow!("invalid task label in package {package}: {e}"))?;

        if let Some(previous) = state.task_origins.get(&label) {
            bail!(
                "duplicate task label: {label}\nfirst defined in {}\nconflicts with {}",
                previous.display(),
                module_path.display()
            );
        }

        let mut deps = Vec::with_capacity(task.deps.len());
        for dep in &task.deps {
            deps.push(parse_label(dep, package).map_err(|e| anyhow!("invalid dep {dep}: {e}"))?);
        }

        let mut needs = task.needs;
        for need in &mut needs {
            need.limiter = with_scope_key(&need.limiter, project_id, root);
        }

        let queue = task
            .queue
            .or_else(|| module.defaults.queue.clone())
            .map(|mut queue_use| {
                queue_use.queue = with_scope_key(&queue_use.queue, project_id, root);
                queue_use
            });

        let retry = task
            .retry
            .or_else(|| module.defaults.retry.clone())
            .unwrap_or_else(RetryDef::default);

        let container_runtime = validate_runtime(
            module.defaults.container_runtime.clone(),
            package,
            "defaults.container_runtime",
        )?;

        let mut tags = module.defaults.tags.clone();
        tags.extend(task.tags);

        let execution = resolve_task_execution(
            task.execution,
            module.defaults.execution.clone(),
            state.default_execution_policy.as_deref(),
            package,
            state,
        )?;
        let context = resolve_current_state(task.context, package)?;
        let outputs = resolve_output_selectors(task.outputs, package)?;

        let resolved = ResolvedTask {
            label: label.clone(),
            doc: task.doc,
            deps,
            steps: task.steps,
            needs,
            queue,
            retry,
            timeout_s: task.timeout_s,
            context,
            outputs,
            container_runtime,
            execution,
            session: None,
            tags,
        };

        state
            .task_origins
            .insert(label.clone(), module_path.to_path_buf());
        state.tasks.insert(label, resolved);
    }

    Ok(())
}
