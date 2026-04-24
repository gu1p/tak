use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use monty::{LimitedTracker, MontyRun, PrintWriter, ResourceLimits};
use tak_core::model::{
    ModuleSpec, PolicyDecisionDef, PolicyDecisionSpec, TaskLabel, WorkspaceSpec,
};

use super::{
    LoadOptions, MergeState, PRELUDE,
    authored_source::{prepare_authored_source, runtime_input_names, runtime_inputs},
    execution_resolution::resolve_policy_decision,
    module_merge::merge_module,
    monty_deserializer::deserialize_from_monty,
    project_resolution::{package_for_file, resolve_project_id},
    session_resolution::bind_task_sessions,
    workspace_discovery::{detect_workspace_root, discover_tasks_files},
};

pub fn load_workspace(root: &Path, options: &LoadOptions) -> Result<WorkspaceSpec> {
    let workspace_root = detect_workspace_root(root)?;
    let discovered = discover_tasks_files(&workspace_root, options)?;
    let mut modules = Vec::<(PathBuf, String, ModuleSpec)>::new();

    for (file, module) in discovered {
        let package = package_for_file(&workspace_root, &file)?;
        modules.push((file, package, module));
    }

    let module_project_ids: Vec<Option<&str>> = modules
        .iter()
        .map(|(_, _, module)| module.project_id.as_deref())
        .collect();
    let project_id = resolve_project_id(
        &workspace_root,
        options.project_id.as_deref(),
        &module_project_ids,
    )?;

    let mut state = MergeState::default();

    for (module_path, package, module) in modules {
        merge_module(
            &module_path,
            &workspace_root,
            &project_id,
            &package,
            module,
            &mut state,
        )?;
    }

    for (label, task) in &state.tasks {
        for dep in &task.deps {
            if !state.tasks.contains_key(dep) {
                bail!("task {label} has unknown dependency {dep}");
            }
        }
    }
    bind_task_sessions(&mut state.tasks, &state.sessions)?;

    let dep_map: BTreeMap<TaskLabel, Vec<TaskLabel>> = state
        .tasks
        .iter()
        .map(|(label, task)| (label.clone(), task.deps.clone()))
        .collect();
    tak_core::planner::topo_sort(&dep_map).context("invalid task graph")?;

    Ok(WorkspaceSpec {
        project_id,
        root: workspace_root,
        tasks: state.tasks,
        sessions: state.sessions,
        limiters: state.limiters,
        queues: state.queues,
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
    package: &str,
    policy_name: &str,
) -> Result<PolicyDecisionSpec> {
    let policy_name = policy_name.trim();
    if policy_name.is_empty() {
        bail!("policy_name is required");
    }

    let source = fs::read_to_string(tasks_file)?;
    let prepared = prepare_authored_source(tasks_file, &source)?;
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

{}

__TAK_RUNTIME_POLICY_CONTEXT__ = POLICY_CONTEXT if isinstance(POLICY_CONTEXT, dict) else PolicyContext()
_compile_policy_decision({policy_name}, __TAK_RUNTIME_POLICY_CONTEXT__)
"#,
        prepared.runtime_source
    );

    let limits = ResourceLimits::new()
        .max_duration(Duration::from_secs(2))
        .max_memory(64 * 1024 * 1024)
        .max_allocations(200_000);
    let tracker = LimitedTracker::new(limits);

    let runner = MontyRun::new(code, &tasks_file.to_string_lossy(), runtime_input_names())
        .map_err(|e| anyhow!("failed to compile {}: {e}", tasks_file.display()))?;
    let value = runner
        .run(runtime_inputs(), tracker, PrintWriter::Disabled)
        .map_err(|e| anyhow!("failed to evaluate {}: {e}", tasks_file.display()))?;

    let decision: PolicyDecisionDef = deserialize_from_monty(value)
        .map_err(|e| anyhow!("invalid policy decision in {}: {e}", tasks_file.display()))?;
    resolve_policy_decision(decision, package)
}
