fn merge_module(
    module_path: &Path,
    root: &Path,
    project_id: &str,
    package: &str,
    module: ModuleSpec,
    state: &mut MergeState,
) -> Result<()> {
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

        let mut tags = module.defaults.tags.clone();
        tags.extend(task.tags);

        let execution = task
            .execution
            .map(resolve_execution)
            .transpose()?
            .unwrap_or_default();
        let context = resolve_current_state(task.context, package)?;

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
            execution,
            tags,
        };

        state
            .task_origins
            .insert(label.clone(), module_path.to_path_buf());
        state.tasks.insert(label, resolved);
    }

    Ok(())
}

/// Copies a limiter reference while resolving the concrete scope key for this workspace.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn with_scope_key(reference: &LimiterRef, project_id: &str, root: &Path) -> LimiterRef {
    LimiterRef {
        name: reference.name.clone(),
        scope: reference.scope.clone(),
        scope_key: scope_key_for(&reference.scope, project_id, root),
    }
}

/// Builds the workspace limiter key for a concrete limiter definition.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn limiter_key_for_limiter(limiter: &LimiterDef, project_id: &str, root: &Path) -> LimiterKey {
    match limiter {
        LimiterDef::Resource { name, scope, .. }
        | LimiterDef::Lock { name, scope }
        | LimiterDef::RateLimit { name, scope, .. }
        | LimiterDef::ProcessCap { name, scope, .. } => LimiterKey {
            scope: scope.clone(),
            scope_key: scope_key_for(scope, project_id, root),
            name: name.clone(),
        },
    }
}

/// Resolves a concrete scope key value for the given scope variant.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn scope_key_for(scope: &Scope, project_id: &str, root: &Path) -> Option<String> {
    match scope {
        Scope::Machine => None,
        Scope::User => env::var("USER")
            .or_else(|_| env::var("USERNAME"))
            .ok()
            .or(Some("unknown".to_string())),
        Scope::Project => Some(project_id.to_string()),
        Scope::Worktree => Some(root.to_string_lossy().into_owned()),
    }
}

fn scope_label(scope: &Scope) -> &'static str {
    match scope {
        Scope::Machine => "machine",
        Scope::User => "user",
        Scope::Project => "project",
        Scope::Worktree => "worktree",
    }
}

fn scope_key_label(scope_key: &Option<String>) -> String {
    scope_key
        .as_deref()
        .map(|value| format!("scope_key={value}"))
        .unwrap_or_else(|| "scope_key=(none)".to_string())
}
