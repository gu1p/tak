pub fn load_workspace(root: &Path, options: &LoadOptions) -> Result<WorkspaceSpec> {
    let workspace_root = detect_workspace_root(root)?;
    let files = discover_tasks_files(&workspace_root)?;
    let mut modules = Vec::<(String, ModuleSpec)>::new();

    for file in files {
        let module = eval_module_spec(&file, options)?;
        let package = package_for_file(&workspace_root, &file)?;
        modules.push((package, module));
    }

    let module_project_ids: Vec<Option<&str>> = modules
        .iter()
        .map(|(_, module)| module.project_id.as_deref())
        .collect();
    let project_id = resolve_project_id(
        &workspace_root,
        options.project_id.as_deref(),
        &module_project_ids,
    )?;

    let mut tasks = BTreeMap::<TaskLabel, ResolvedTask>::new();
    let mut limiters = HashMap::<LimiterKey, LimiterDef>::new();
    let mut queues = HashMap::<LimiterKey, QueueDef>::new();

    for (package, module) in modules {
        merge_module(
            &workspace_root,
            &project_id,
            &package,
            module,
            &mut tasks,
            &mut limiters,
            &mut queues,
        )?;
    }

    for (label, task) in &tasks {
        for dep in &task.deps {
            if !tasks.contains_key(dep) {
                bail!("task {label} has unknown dependency {dep}");
            }
        }
    }

    let dep_map: BTreeMap<TaskLabel, Vec<TaskLabel>> = tasks
        .iter()
        .map(|(label, task)| (label.clone(), task.deps.clone()))
        .collect();
    tak_core::planner::topo_sort(&dep_map).context("invalid task graph")?;

    Ok(WorkspaceSpec {
        project_id,
        root: workspace_root,
        tasks,
        limiters,
        queues,
    })
}

/// Evaluates a named policy function from one `TASKS.py` file and resolves it to V1 policy IR.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub fn evaluate_named_policy_decision(
    tasks_file: &Path,
    policy_name: &str,
) -> Result<PolicyDecisionSpec> {
    let policy_name = policy_name.trim();
    if policy_name.is_empty() {
        bail!("policy_name is required");
    }

    let source = fs::read_to_string(tasks_file)?;
    let source = sanitize_canonical_v1_imports(&source);
    let mut chars = policy_name.chars();
    let Some(first_char) = chars.next() else {
        bail!("policy_name is required");
    };
    if !(first_char == '_' || first_char.is_ascii_alphabetic())
        || !chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
    {
        bail!("policy_name must be a valid identifier");
    }
    let code = format!(
        r#"{PRELUDE}

{source}

__TAK_RUNTIME_POLICY_CONTEXT__ = POLICY_CONTEXT if isinstance(POLICY_CONTEXT, dict) else PolicyContext()
_compile_policy_decision({policy_name}, __TAK_RUNTIME_POLICY_CONTEXT__)
"#
    );

    let limits = ResourceLimits::new()
        .max_duration(Duration::from_secs(2))
        .max_memory(64 * 1024 * 1024)
        .max_allocations(200_000);
    let tracker = LimitedTracker::new(limits);

    let runner = MontyRun::new(code, &tasks_file.to_string_lossy(), Vec::new(), Vec::new())
        .map_err(|e| anyhow!("failed to compile {}: {e}", tasks_file.display()))?;
    let value = runner
        .run(Vec::new(), tracker, &mut PrintWriter::Disabled)
        .map_err(|e| anyhow!("failed to evaluate {}: {e}", tasks_file.display()))?;

    let json = monty_to_json(value)?;
    let decision: PolicyDecisionDef = serde_json::from_value(json)
        .map_err(|e| anyhow!("invalid policy decision in {}: {e}", tasks_file.display()))?;
    resolve_policy_decision(decision)
}
