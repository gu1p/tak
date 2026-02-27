//! Workspace discovery and `TASKS.py` loading.
//!
//! This crate discovers task definition files, evaluates them via Monty, converts output
//! into strongly-typed core models, and assembles a resolved `WorkspaceSpec`.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use ignore::WalkBuilder;
use monty::{LimitedTracker, MontyObject, MontyRun, PrintWriter, ResourceLimits};
use monty_type_checking::{SourceFile, type_check};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use tak_core::label::parse_label;
use tak_core::model::{
    CurrentStateDef, CurrentStateSpec, IgnoreSourceDef, IgnoreSourceSpec, LimiterDef, LimiterKey,
    LimiterRef, LocalSpec, ModuleSpec, PathInputDef, PolicyDecisionDef, PolicyDecisionModeDef,
    PolicyDecisionSpec, QueueDef, RemoteDef, RemoteResultDef, RemoteRuntimeDef, RemoteRuntimeSpec,
    RemoteSelectionDef, RemoteSelectionSpec, RemoteSpec, RemoteTransportDef, RemoteTransportKind,
    RemoteWorkspaceDef, ResolvedTask, RetryDef, Scope, ServiceAuthDef, TaskExecutionDef,
    TaskExecutionSpec, TaskLabel, WorkspaceSpec, normalize_path_ref,
    validate_container_runtime_execution_spec,
};

const TASKS_FILE: &str = "TASKS.py";
const V1_TRANSPORT_DIRECT_HTTPS: &str = "direct_https";
const V1_TRANSPORT_TOR: &str = "tor";
const V1_TRANSPORT_AUTH_FROM_ENV: &str = "from_env";
const V1_WORKSPACE_TRANSFER_MODE: &str = "REPO_ZIP_SNAPSHOT";
const V1_RESULT_SYNC_MODE: &str = "OUTPUTS_AND_LOGS";

#[derive(Debug, Clone)]
pub struct LoadOptions {
    pub enable_type_check: bool,
    pub project_id: Option<String>,
}

impl Default for LoadOptions {
    /// Creates loader options with type checking enabled and no forced project id.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    fn default() -> Self {
        Self {
            enable_type_check: true,
            project_id: None,
        }
    }
}

/// Detects the workspace root based on `.git` or the current path.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub fn detect_workspace_root(start: &Path) -> Result<PathBuf> {
    let start = start.canonicalize().unwrap_or_else(|_| start.to_path_buf());
    let search_start = if start.is_file() {
        start
            .parent()
            .map_or_else(|| start.clone(), Path::to_path_buf)
    } else {
        start
    };

    if let Some(git) = find_ancestor_with(&search_start, ".git") {
        return Ok(git);
    }
    Ok(search_start)
}

/// Recursively discovers `TASKS.py` files while honoring gitignore-style filters.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub fn discover_tasks_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut builder = WalkBuilder::new(root);
    builder
        .git_ignore(true)
        .git_exclude(true)
        .parents(true)
        .require_git(false)
        .hidden(false)
        .ignore(true);

    for entry in builder.build() {
        let entry = entry?;
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }
        if entry
            .path()
            .file_name()
            .is_some_and(|name| name == TASKS_FILE)
        {
            files.push(entry.into_path());
        }
    }

    files.sort();
    Ok(files)
}

/// Loads and resolves all task modules into a single validated `WorkspaceSpec`.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
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

/// Finds the nearest ancestor containing `marker`.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn find_ancestor_with(start: &Path, marker: &str) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|p| p.join(marker).exists())
        .map(Path::to_path_buf)
}

/// Resolves the project id from options, `TASKS.py` module specs, or a path-based hash fallback.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn resolve_project_id(
    root: &Path,
    from_options: Option<&str>,
    from_modules: &[Option<&str>],
) -> Result<String> {
    if let Some(value) = from_options {
        if value.trim().is_empty() {
            bail!("project_id from options cannot be empty");
        }
        return Ok(value.to_string());
    }

    let mut module_ids = BTreeSet::new();
    for value in from_modules.iter().flatten() {
        let normalized = value.trim();
        if normalized.is_empty() {
            bail!("project_id in TASKS.py cannot be empty");
        }
        module_ids.insert(normalized.to_string());
    }

    if module_ids.len() > 1 {
        let ids = module_ids.into_iter().collect::<Vec<_>>().join(", ");
        bail!("conflicting project_id values in TASKS.py modules: {ids}");
    }

    if let Some(project_id) = module_ids.iter().next() {
        return Ok(project_id.clone());
    }

    let canonical = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let mut hasher = Sha256::new();
    hasher.update(canonical.to_string_lossy().as_bytes());
    let digest = hasher.finalize();
    Ok(format!("project-{}", hex::encode(&digest[..8])))
}

/// Converts a discovered `TASKS.py` path into a Tak package label prefix.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn package_for_file(root: &Path, tasks_file: &Path) -> Result<String> {
    let parent = tasks_file
        .parent()
        .ok_or_else(|| anyhow!("TASKS.py has no parent: {}", tasks_file.display()))?;
    let relative = parent.strip_prefix(root).map_err(|e| {
        anyhow!(
            "{} is outside root {}: {e}",
            parent.display(),
            root.display()
        )
    })?;

    if relative.as_os_str().is_empty() {
        return Ok("//".to_string());
    }

    let mut label = String::from("//");
    let mut first = true;
    for component in relative.components() {
        if !first {
            label.push('/');
        }
        first = false;
        label.push_str(&component.as_os_str().to_string_lossy());
    }
    Ok(label)
}

/// Merges one `ModuleSpec` into the workspace registries and resolves task-local defaults.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn merge_module(
    root: &Path,
    project_id: &str,
    package: &str,
    module: ModuleSpec,
    tasks: &mut BTreeMap<TaskLabel, ResolvedTask>,
    limiters: &mut HashMap<LimiterKey, LimiterDef>,
    queues: &mut HashMap<LimiterKey, QueueDef>,
) -> Result<()> {
    for limiter in module.limiters {
        let key = limiter_key_for_limiter(&limiter, project_id, root);
        if limiters.insert(key.clone(), limiter).is_some() {
            bail!("duplicate limiter definition: {}", key.name);
        }
    }

    for queue in module.queues {
        let key = LimiterKey {
            scope: queue.scope.clone(),
            scope_key: scope_key_for(&queue.scope, project_id, root),
            name: queue.name.clone(),
        };
        if queues.insert(key.clone(), queue).is_some() {
            bail!("duplicate queue definition: {}", key.name);
        }
    }

    for task in module.tasks {
        let label = parse_label(&format!("{package}:{}", task.name), package)
            .map_err(|e| anyhow!("invalid task label in package {package}: {e}"))?;

        if tasks.contains_key(&label) {
            bail!("duplicate task label: {label}");
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

        tasks.insert(label, resolved);
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

/// Resolves one task context block into normalized workspace-anchored `CurrentStateSpec`.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn resolve_current_state(
    context: Option<CurrentStateDef>,
    package: &str,
) -> Result<CurrentStateSpec> {
    let Some(context) = context else {
        return Ok(CurrentStateSpec::default());
    };

    let mut roots = Vec::new();
    for root in context.roots {
        roots.push(resolve_context_path(root, package)?);
    }
    if roots.is_empty() {
        roots.push(
            normalize_path_ref("workspace", ".")
                .map_err(|e| anyhow!("invalid default workspace root path: {e}"))?,
        );
    }

    let mut ignored = Vec::new();
    for source in context.ignored {
        ignored.push(resolve_ignore_source(source, package)?);
    }

    let mut include = Vec::new();
    for path in context.include {
        include.push(resolve_context_path(path, package)?);
    }

    Ok(CurrentStateSpec {
        roots,
        ignored,
        include,
    })
}

/// Resolves an ignore source entry to the internal typed representation.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn resolve_ignore_source(source: IgnoreSourceDef, package: &str) -> Result<IgnoreSourceSpec> {
    match source {
        IgnoreSourceDef::Path { value } => {
            let resolved = resolve_context_path(PathInputDef::Path { value }, package)?;
            Ok(IgnoreSourceSpec::Path(resolved))
        }
        IgnoreSourceDef::Gitignore => Ok(IgnoreSourceSpec::GitIgnore),
    }
}

/// Resolves one declared context path into a canonical workspace-anchored `PathRef`.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn resolve_context_path(path: PathInputDef, package: &str) -> Result<tak_core::model::PathRef> {
    let raw = match path {
        PathInputDef::Path { value } => value.trim().to_string(),
    };
    if raw.is_empty() {
        bail!("context path cannot be empty");
    }

    if let Some(workspace_relative) = raw.strip_prefix("//") {
        return normalize_path_ref("workspace", workspace_relative)
            .map_err(|e| anyhow!("invalid workspace context path `{raw}`: {e}"));
    }

    if raw.starts_with('@') {
        bail!("context repo anchors are not supported yet in V1: {raw}");
    }

    let package_relative = package.trim_start_matches("//");
    let joined = if package_relative.is_empty() {
        raw.to_string()
    } else {
        format!("{package_relative}/{raw}")
    };
    normalize_path_ref("workspace", &joined)
        .map_err(|e| anyhow!("invalid package context path `{raw}`: {e}"))
}

/// Validates and resolves a task execution declaration into canonical runtime form.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn resolve_execution(execution: TaskExecutionDef) -> Result<TaskExecutionSpec> {
    match execution {
        TaskExecutionDef::LocalOnly { local } => {
            let id = local.id.trim().to_string();
            if id.is_empty() {
                bail!("execution LocalOnly.local.id cannot be empty");
            }
            if local.max_parallel_tasks == 0 {
                bail!("execution LocalOnly.local.max_parallel_tasks must be >= 1");
            }
            Ok(TaskExecutionSpec::LocalOnly(LocalSpec {
                id,
                max_parallel_tasks: local.max_parallel_tasks,
            }))
        }
        TaskExecutionDef::RemoteOnly { remote } => Ok(TaskExecutionSpec::RemoteOnly(
            resolve_remote_selection(remote)?,
        )),
        TaskExecutionDef::ByCustomPolicy {
            policy_name,
            decision,
        } => {
            let policy_name = policy_name.trim().to_string();
            if policy_name.is_empty() {
                bail!("execution ByCustomPolicy.policy_name cannot be empty");
            }
            let decision = decision.map(resolve_policy_decision).transpose()?;
            Ok(TaskExecutionSpec::ByCustomPolicy {
                policy_name,
                decision,
            })
        }
    }
}

/// Resolves a policy-produced execution decision into strict runtime form.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn resolve_policy_decision(decision: PolicyDecisionDef) -> Result<PolicyDecisionSpec> {
    let reason = {
        let normalized = decision.reason.trim().to_string();
        if normalized.is_empty() {
            "DEFAULT_LOCAL_POLICY".to_string()
        } else {
            normalized
        }
    };

    match decision.mode {
        PolicyDecisionModeDef::Local => {
            if decision.remote.is_some() || !decision.remotes.is_empty() {
                bail!("execution ByCustomPolicy.decision local mode cannot include remote targets");
            }
            Ok(PolicyDecisionSpec::Local { reason })
        }
        PolicyDecisionModeDef::Remote => {
            if !decision.remotes.is_empty() {
                bail!("execution ByCustomPolicy.decision remote mode cannot include remotes list");
            }
            let remote = decision.remote.ok_or_else(|| {
                anyhow!("execution ByCustomPolicy.decision remote mode requires remote")
            })?;
            let remote = resolve_remote(*remote)?;
            if remote.endpoint.is_none() {
                bail!(
                    "execution ByCustomPolicy.decision remote target {} requires endpoint",
                    remote.id
                );
            }
            Ok(PolicyDecisionSpec::Remote { reason, remote })
        }
        PolicyDecisionModeDef::RemoteAny => {
            if decision.remote.is_some() {
                bail!(
                    "execution ByCustomPolicy.decision remote_any mode cannot include singular remote"
                );
            }
            if decision.remotes.is_empty() {
                bail!(
                    "execution ByCustomPolicy.decision remote_any mode requires non-empty remotes"
                );
            }

            let mut remotes = Vec::with_capacity(decision.remotes.len());
            for remote in decision.remotes {
                let resolved = resolve_remote(remote)?;
                if resolved.endpoint.is_none() {
                    bail!(
                        "execution ByCustomPolicy.decision remote_any target {} requires endpoint",
                        resolved.id
                    );
                }
                remotes.push(resolved);
            }

            Ok(PolicyDecisionSpec::RemoteAny { reason, remotes })
        }
    }
}

/// Resolves one remote selection shape while enforcing non-empty node ids.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn resolve_remote_selection(selection: RemoteSelectionDef) -> Result<RemoteSelectionSpec> {
    match selection {
        RemoteSelectionDef::Single(remote) => {
            Ok(RemoteSelectionSpec::Single(resolve_remote(*remote)?))
        }
        RemoteSelectionDef::List(remotes) => {
            if remotes.is_empty() {
                bail!("execution RemoteOnly.remote list cannot be empty");
            }
            let mut resolved = Vec::with_capacity(remotes.len());
            for remote in remotes {
                resolved.push(resolve_remote(remote)?);
            }
            Ok(RemoteSelectionSpec::List(resolved))
        }
    }
}

/// Resolves one remote node declaration used by task execution selectors.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn resolve_remote(remote: RemoteDef) -> Result<RemoteSpec> {
    let RemoteDef {
        id: raw_id,
        endpoint: raw_endpoint,
        transport,
        workspace,
        result,
        runtime,
    } = remote;

    let id = raw_id.trim().to_string();
    if id.is_empty() {
        bail!("execution Remote.id cannot be empty");
    }
    let endpoint = raw_endpoint.and_then(|value| {
        let normalized = value.trim().to_string();
        if normalized.is_empty() {
            None
        } else {
            Some(normalized)
        }
    });

    let (transport_kind, service_auth_env) = validate_remote_transport(transport)?;
    validate_remote_workspace(workspace)?;
    validate_remote_result(result)?;
    let runtime = validate_remote_runtime(runtime)?;

    Ok(RemoteSpec {
        id,
        endpoint,
        transport_kind,
        service_auth_env,
        runtime,
    })
}

fn validate_remote_transport(
    transport: Option<RemoteTransportDef>,
) -> Result<(RemoteTransportKind, Option<String>)> {
    let Some(transport) = transport else {
        return Ok((RemoteTransportKind::DirectHttps, None));
    };

    let service_auth_env = validate_service_auth(transport.auth)?;
    let kind = transport.kind.trim();
    if kind.is_empty() {
        bail!("execution Remote.transport.kind cannot be empty");
    }

    match kind {
        V1_TRANSPORT_DIRECT_HTTPS => Ok((RemoteTransportKind::DirectHttps, service_auth_env)),
        V1_TRANSPORT_TOR => Ok((RemoteTransportKind::Tor, service_auth_env)),
        _ => bail!(
            "execution Remote.transport.kind `{kind}` is unsupported in V1; expected `{V1_TRANSPORT_DIRECT_HTTPS}` or `{V1_TRANSPORT_TOR}`"
        ),
    }
}

fn validate_service_auth(auth: Option<ServiceAuthDef>) -> Result<Option<String>> {
    let Some(auth) = auth else {
        return Ok(None);
    };

    let kind = auth.kind.trim();
    if kind != V1_TRANSPORT_AUTH_FROM_ENV {
        bail!(
            "execution Remote.transport.auth.kind `{kind}` is unsupported in V1; expected `{V1_TRANSPORT_AUTH_FROM_ENV}`"
        );
    }

    let env_name = auth
        .env_name
        .as_deref()
        .map(str::trim)
        .unwrap_or_default()
        .to_string();
    if env_name.is_empty() {
        bail!("execution Remote.transport.auth.env_name cannot be empty");
    }

    Ok(Some(env_name))
}

fn validate_remote_workspace(workspace: Option<RemoteWorkspaceDef>) -> Result<()> {
    let Some(workspace) = workspace else {
        return Ok(());
    };

    let transfer = workspace.transfer.trim();
    if transfer != V1_WORKSPACE_TRANSFER_MODE {
        bail!("execution Remote.workspace.transfer must be `{V1_WORKSPACE_TRANSFER_MODE}` in V1");
    }

    Ok(())
}

fn validate_remote_result(result: Option<RemoteResultDef>) -> Result<()> {
    let Some(result) = result else {
        return Ok(());
    };

    let sync = result.sync.trim();
    if sync != V1_RESULT_SYNC_MODE {
        bail!("execution Remote.result.sync must be `{V1_RESULT_SYNC_MODE}` in V1");
    }

    Ok(())
}

fn validate_remote_runtime(runtime: Option<RemoteRuntimeDef>) -> Result<Option<RemoteRuntimeSpec>> {
    let Some(runtime) = runtime else {
        return Ok(None);
    };

    let validated = validate_container_runtime_execution_spec(&runtime)
        .map_err(|err| anyhow!("execution Remote.{err}"))?;
    let image = validated.image;

    Ok(Some(RemoteRuntimeSpec::Containerized { image }))
}

/// Evaluates a single `TASKS.py` file in Monty and deserializes it into `ModuleSpec`.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn eval_module_spec(path: &Path, options: &LoadOptions) -> Result<ModuleSpec> {
    let source = fs::read_to_string(path)?;
    let source = sanitize_canonical_v1_imports(&source);
    let code = format!("{PRELUDE}\n\n{source}");

    if options.enable_type_check {
        let script_name = path.to_string_lossy();
        let source = SourceFile::new(&code, &script_name);
        let stubs = SourceFile::new(DSL_STUBS, "tak_dsl.pyi");
        match type_check(&source, Some(&stubs)) {
            Ok(None) => {}
            Ok(Some(diagnostics)) => {
                bail!("type errors in {}:\n{}", path.display(), diagnostics);
            }
            Err(err) => {
                bail!("type-checking failed for {}: {err}", path.display());
            }
        }
    }

    let limits = ResourceLimits::new()
        .max_duration(Duration::from_secs(2))
        .max_memory(64 * 1024 * 1024)
        .max_allocations(200_000);
    let tracker = LimitedTracker::new(limits);

    let runner = MontyRun::new(code, &path.to_string_lossy(), Vec::new(), Vec::new())
        .map_err(|e| anyhow!("failed to compile {}: {e}", path.display()))?;
    let value = runner
        .run(Vec::new(), tracker, &mut PrintWriter::Disabled)
        .map_err(|e| anyhow!("failed to evaluate {}: {e}", path.display()))?;

    let json = monty_to_json(value)?;
    let module: ModuleSpec = serde_json::from_value(json)
        .map_err(|e| anyhow!("invalid module spec in {}: {e}", path.display()))?;

    if module.spec_version != 1 {
        bail!(
            "unsupported spec_version {} in {}",
            module.spec_version,
            path.display()
        );
    }

    Ok(module)
}

fn sanitize_canonical_v1_imports(source: &str) -> String {
    let mut output = Vec::new();
    let mut skipping_multiline_import = false;

    for line in source.lines() {
        let trimmed = line.trim();
        if skipping_multiline_import {
            if trimmed.contains(')') {
                skipping_multiline_import = false;
            }
            continue;
        }

        let is_tak_import =
            trimmed.starts_with("from tak import") || trimmed.starts_with("from tak.remote import");
        if is_tak_import {
            if trimmed.contains('(') && !trimmed.contains(')') {
                skipping_multiline_import = true;
            }
            continue;
        }

        output.push(line);
    }

    let mut normalized = output.join("\n");
    normalized = normalized
        .replace("RemoteTransportMode.DirectHttps(", "DirectHttps(")
        .replace("RemoteTransportMode.TorOnionService(", "TorOnionService(")
        .replace("ServiceAuth.from_env(", "ServiceAuth_from_env(")
        .replace(
            "WorkspaceTransferMode.REPO_ZIP_SNAPSHOT",
            "\"REPO_ZIP_SNAPSHOT\"",
        )
        .replace("ResultSyncMode.OUTPUTS_AND_LOGS", "\"OUTPUTS_AND_LOGS\"")
        .replace("Decision.remote_any(", "Decision_remote_any(")
        .replace("Decision.remote(", "Decision_remote(")
        .replace("Decision.local(", "Decision_local(")
        .replace("Reason.SIDE_EFFECTING_TASK", "REASON_SIDE_EFFECTING_TASK")
        .replace("Reason.NO_REMOTE_REACHABLE", "REASON_NO_REMOTE_REACHABLE")
        .replace(
            "Reason.LOCAL_CPU_HIGH_ARM_IDLE",
            "REASON_LOCAL_CPU_HIGH_ARM_IDLE",
        )
        .replace("Reason.LOCAL_CPU_HIGH", "REASON_LOCAL_CPU_HIGH")
        .replace("Reason.DEFAULT_LOCAL_POLICY", "REASON_DEFAULT_LOCAL_POLICY");
    if source.ends_with('\n') {
        normalized.push('\n');
    }
    normalized
}

/// Converts a Monty runtime object into strict JSON-compatible `serde_json::Value`.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn monty_to_json(value: MontyObject) -> Result<Value> {
    let json = match value {
        MontyObject::None => Value::Null,
        MontyObject::Bool(v) => Value::Bool(v),
        MontyObject::Int(v) => Value::Number(v.into()),
        MontyObject::BigInt(v) => {
            let as_i64 = v
                .to_string()
                .parse::<i64>()
                .map_err(|_| anyhow!("bigint value out of i64 range"))?;
            Value::Number(as_i64.into())
        }
        MontyObject::Float(v) => {
            let number = serde_json::Number::from_f64(v)
                .ok_or_else(|| anyhow!("non-finite float value is not JSON-compatible"))?;
            Value::Number(number)
        }
        MontyObject::String(v) => Value::String(v),
        MontyObject::List(items) => Value::Array(
            items
                .into_iter()
                .map(monty_to_json)
                .collect::<Result<Vec<_>>>()?,
        ),
        MontyObject::Dict(pairs) => {
            let mut map = Map::new();
            for (key, value) in pairs {
                let key_string = match key {
                    MontyObject::String(s) => s,
                    other => return Err(anyhow!("dict key must be a string, got {other:?}")),
                };
                map.insert(key_string, monty_to_json(value)?);
            }
            Value::Object(map)
        }
        other => return Err(anyhow!("non-JSON-compatible Monty value: {other:?}")),
    };

    Ok(json)
}

const PRELUDE: &str = r#"
MACHINE  = "machine"
USER     = "user"
PROJECT  = "project"
WORKTREE = "worktree"

DURING   = "during"
AT_START = "at_start"

FIFO     = "fifo"
PRIORITY = "priority"

def _or_empty_list(value):
    return value if value is not None else []

def _or_empty_dict(value):
    return value if value is not None else {}

def _dep_to_label(value):
    if isinstance(value, str):
        return value
    if isinstance(value, dict):
        name = value.get("name")
        if isinstance(name, str):
            if name.startswith("//") or name.startswith(":"):
                return name
            return ":" + name
    raise TypeError("dependency must be a label string or a task object")

def _normalize_deps(value):
    if value is None:
        return []
    if isinstance(value, list):
        return [_dep_to_label(item) for item in value]
    return [_dep_to_label(value)]

def module_spec(tasks, limiters=None, queues=None, exclude=None, defaults=None, project_id=None):
    return {
        "spec_version": 1,
        "project_id": project_id,
        "tasks": tasks,
        "limiters": _or_empty_list(limiters),
        "queues": _or_empty_list(queues),
        "exclude": _or_empty_list(exclude),
        "defaults": defaults if defaults is not None else {},
    }

def Local(id, max_parallel_tasks=1):
    return {
        "id": id,
        "max_parallel_tasks": max_parallel_tasks,
    }

def Remote(id, endpoint=None, transport=None, workspace=None, result=None, runtime=None):
    normalized_transport = transport
    normalized_endpoint = endpoint
    normalized_workspace = workspace
    normalized_result = result
    if isinstance(transport, dict):
        normalized_endpoint = endpoint if endpoint is not None else transport.get("endpoint")
        normalized_transport = {"kind": transport.get("kind"), "auth": transport.get("auth")}
    if isinstance(workspace, str):
        normalized_workspace = {"transfer": workspace}
    if isinstance(result, str):
        normalized_result = {"sync": result}
    return {
        "id": id,
        "endpoint": normalized_endpoint,
        "transport": normalized_transport,
        "workspace": normalized_workspace,
        "result": normalized_result,
        "runtime": runtime,
    }

def DirectHttps(endpoint, auth=None):
    return {
        "kind": "direct_https",
        "endpoint": str(endpoint),
        "auth": auth,
    }

def TorOnionService(endpoint, auth=None):
    return {
        "kind": "tor",
        "endpoint": str(endpoint),
        "auth": auth,
    }

def ServiceAuth_from_env(env_name):
    return {
        "kind": "from_env",
        "env_name": str(env_name),
    }

REPO_ZIP_SNAPSHOT = "REPO_ZIP_SNAPSHOT"
OUTPUTS_AND_LOGS = "OUTPUTS_AND_LOGS"

def results(sync=OUTPUTS_AND_LOGS):
    return {
        "sync": str(sync),
    }

def ContainerRuntime(image, command=None, mounts=None, env=None, resources=None):
    return {
        "kind": "containerized",
        "image": str(image),
        "command": _or_empty_list(command) if command is not None else None,
        "mounts": _or_empty_list(mounts),
        "env": _or_empty_dict(env),
        "resource_limits": resources,
    }

REASON_SIDE_EFFECTING_TASK = "SIDE_EFFECTING_TASK"
REASON_NO_REMOTE_REACHABLE = "NO_REMOTE_REACHABLE"
REASON_LOCAL_CPU_HIGH_ARM_IDLE = "LOCAL_CPU_HIGH_ARM_IDLE"
REASON_LOCAL_CPU_HIGH = "LOCAL_CPU_HIGH"
REASON_DEFAULT_LOCAL_POLICY = "DEFAULT_LOCAL_POLICY"

Reason = {
    "SIDE_EFFECTING_TASK": REASON_SIDE_EFFECTING_TASK,
    "NO_REMOTE_REACHABLE": REASON_NO_REMOTE_REACHABLE,
    "LOCAL_CPU_HIGH_ARM_IDLE": REASON_LOCAL_CPU_HIGH_ARM_IDLE,
    "LOCAL_CPU_HIGH": REASON_LOCAL_CPU_HIGH,
    "DEFAULT_LOCAL_POLICY": REASON_DEFAULT_LOCAL_POLICY,
}

def RemoteRuntimeView(endpoint=None, healthy=False, queue_eta_s=0.0):
    return {
        "endpoint": str(endpoint) if endpoint is not None else None,
        "healthy": bool(healthy),
        "queue_eta_s": float(queue_eta_s),
    }

def PolicyContext(
    task_side_effecting=False,
    local_cpu_percent=0.0,
    remotes=None,
    remote_any_reachable=None,
):
    resolved_remotes = dict(remotes) if remotes is not None else {}
    if remote_any_reachable is None:
        reachable = len(resolved_remotes) > 0
    else:
        reachable = bool(remote_any_reachable)

    return {
        "task": {"side_effecting": bool(task_side_effecting)},
        "local": {"cpu_percent": float(local_cpu_percent)},
        "remote_any_reachable": reachable,
        "remotes": resolved_remotes,
    }

def policy_remote(ctx, node_id):
    remotes = ctx.get("remotes")
    if not isinstance(remotes, dict):
        return None
    return remotes.get(str(node_id))

def Decision_local(reason=REASON_DEFAULT_LOCAL_POLICY):
    return {
        "mode": "local",
        "reason": str(reason),
    }

def Decision_remote(node_id, reason="DEFAULT_REMOTE_POLICY"):
    return {
        "mode": "remote",
        "node_id": str(node_id),
        "reason": str(reason),
    }

def Decision_remote_any(node_ids, reason="DEFAULT_REMOTE_ANY_POLICY"):
    return {
        "mode": "remote_any",
        "node_ids": [str(node_id) for node_id in node_ids],
        "reason": str(reason),
    }

def _unsupported_policy_builder_api(name):
    raise TypeError(
        "unsupported policy builder API: "
        + str(name)
        + " (use Decision.local/Decision.remote/Decision.remote_any)"
    )

def Decision_start(*args, **kwargs):
    _unsupported_policy_builder_api("Decision.start")

def Decision_prefer_local(*args, **kwargs):
    _unsupported_policy_builder_api("Decision.prefer_local")

def Decision_prefer_remote(*args, **kwargs):
    _unsupported_policy_builder_api("Decision.prefer_remote")

def Decision_resolve(*args, **kwargs):
    _unsupported_policy_builder_api("Decision.resolve")

POLICY_CONTEXT = PolicyContext()

def _is_local_constructor_value(value):
    return (
        isinstance(value, dict)
        and "id" in value
        and "max_parallel_tasks" in value
        and "endpoint" not in value
    )

def _is_remote_constructor_value(value):
    return (
        isinstance(value, dict)
        and "id" in value
        and "max_parallel_tasks" not in value
    )

def LocalOnly(local):
    if not _is_local_constructor_value(local):
        raise TypeError("LocalOnly expects Local(...)")
    return {
        "kind": "local_only",
        "local": local,
    }

def RemoteOnly(remote):
    if isinstance(remote, list):
        for node in remote:
            if not _is_remote_constructor_value(node):
                raise TypeError("RemoteOnly expects Remote(...) or list[Remote(...)]")
    elif not _is_remote_constructor_value(remote):
        raise TypeError("RemoteOnly expects Remote(...) or list[Remote(...)]")
    return {
        "kind": "remote_only",
        "remote": remote,
    }

def _resolve_policy_remote(context, node_id):
    remote_view = policy_remote(context, node_id)
    if not isinstance(remote_view, dict):
        raise TypeError("policy decision references unknown remote node: " + str(node_id))
    endpoint = remote_view.get("endpoint")
    if endpoint is None or str(endpoint).strip() == "":
        raise TypeError("policy decision node is missing endpoint: " + str(node_id))
    return {
        "id": str(node_id),
        "endpoint": str(endpoint),
    }

def _compile_policy_decision(policy, context):
    decision = policy(context)
    if not isinstance(decision, dict):
        raise TypeError("policy function must return Decision.local/remote/remote_any")

    scoring_fields = []
    if "score" in decision:
        scoring_fields.append("score")
    if "weight" in decision:
        scoring_fields.append("weight")
    if len(scoring_fields) > 0:
        raise TypeError(
            "unsupported policy scoring fields: " + ", ".join(scoring_fields)
        )

    mode = decision.get("mode")
    reason = str(decision.get("reason", REASON_DEFAULT_LOCAL_POLICY))

    if mode == "local":
        return {
            "mode": "local",
            "reason": reason,
        }

    if mode == "remote":
        node_id = decision.get("node_id")
        if node_id is None or str(node_id).strip() == "":
            raise TypeError("Decision.remote requires non-empty node_id")
        return {
            "mode": "remote",
            "reason": reason,
            "remote": _resolve_policy_remote(context, node_id),
        }

    if mode == "remote_any":
        node_ids = decision.get("node_ids")
        if not isinstance(node_ids, list) or len(node_ids) == 0:
            raise TypeError("Decision.remote_any requires non-empty node_ids")
        return {
            "mode": "remote_any",
            "reason": reason,
            "remotes": [_resolve_policy_remote(context, node_id) for node_id in node_ids],
        }

    raise TypeError("unsupported policy decision mode: " + str(mode))

def ByCustomPolicy(policy):
    if not isinstance(POLICY_CONTEXT, dict):
        raise TypeError("POLICY_CONTEXT must be PolicyContext(...)")

    if not isinstance(policy, str):
        decision = _compile_policy_decision(policy, POLICY_CONTEXT)
        return {
            "kind": "by_custom_policy",
            "policy_name": str(policy),
            "decision": decision,
        }
    return {
        "kind": "by_custom_policy",
        "policy_name": str(policy),
    }

def path(value):
    return {
        "kind": "path",
        "value": str(value),
    }

def gitignore():
    return {
        "kind": "gitignore",
    }

def CurrentState(roots=None, ignored=None, include=None):
    return {
        "roots": _or_empty_list(roots),
        "ignored": _or_empty_list(ignored),
        "include": _or_empty_list(include),
    }

def task(name, deps=None, steps=None, needs=None, queue=None, retry=None, timeout_s=None, context=None, execution=None, tags=None, doc=None):
    return {
        "name": name,
        "deps": _normalize_deps(deps),
        "steps": _or_empty_list(steps),
        "needs": _or_empty_list(needs),
        "queue": queue,
        "retry": retry,
        "timeout_s": timeout_s,
        "context": context,
        "execution": execution,
        "tags": _or_empty_list(tags),
        "doc": doc if doc is not None else "",
    }

def cmd(*argv, cwd=None, env=None):
    return {
        "kind": "cmd",
        "argv": list(argv),
        "cwd": cwd,
        "env": _or_empty_dict(env),
    }

def script(path, *argv, interpreter=None, cwd=None, env=None):
    return {
        "kind": "script",
        "path": path,
        "argv": list(argv),
        "interpreter": interpreter,
        "cwd": cwd,
        "env": _or_empty_dict(env),
    }

def need(name, slots=1, scope=PROJECT, hold=DURING):
    return {
        "limiter": {"name": name, "scope": scope},
        "slots": slots,
        "hold": hold,
    }

def queue_use(name, scope=MACHINE, slots=1, priority=0):
    return {
        "queue": {"name": name, "scope": scope},
        "slots": slots,
        "priority": priority,
    }

def resource(name, capacity, unit=None, scope=MACHINE):
    return {
        "kind": "resource",
        "name": name,
        "scope": scope,
        "capacity": capacity,
        "unit": unit,
    }

def lock(name, scope=MACHINE):
    return {
        "kind": "lock",
        "name": name,
        "scope": scope,
    }

def queue_def(name, slots, discipline=FIFO, max_pending=None, scope=MACHINE):
    return {
        "name": name,
        "scope": scope,
        "slots": slots,
        "discipline": discipline,
        "max_pending": max_pending,
    }

def rate_limit(name, burst, refill_per_second, scope=MACHINE):
    return {
        "kind": "rate_limit",
        "name": name,
        "scope": scope,
        "burst": burst,
        "refill_per_second": refill_per_second,
    }

def process_cap(name, max_running, match=None, scope=MACHINE):
    return {
        "kind": "process_cap",
        "name": name,
        "scope": scope,
        "max_running": max_running,
        "match": match,
    }

def retry(attempts=1, on_exit=None, backoff=None):
    return {
        "attempts": attempts,
        "on_exit": _or_empty_list(on_exit),
        "backoff": backoff if backoff is not None else fixed(0),
    }

def fixed(seconds):
    return {
        "kind": "fixed",
        "seconds": seconds,
    }

def exp_jitter(min_s=1, max_s=60, jitter="full"):
    return {
        "kind": "exp_jitter",
        "min_s": min_s,
        "max_s": max_s,
        "jitter": jitter,
    }
"#;

const DSL_STUBS: &str = r#"
MACHINE: str
USER: str
PROJECT: str
WORKTREE: str
DURING: str
AT_START: str
FIFO: str
PRIORITY: str

def module_spec(tasks: list, limiters: list | None = ..., queues: list | None = ..., exclude: list | None = ..., defaults: dict | None = ..., project_id: str | None = ...) -> dict: ...
def task(name: str, deps: list | str | dict | None = ..., steps: list | None = ..., needs: list | None = ..., queue: dict | None = ..., retry: dict | None = ..., timeout_s: int | None = ..., context: dict | None = ..., execution: dict | None = ..., tags: list | None = ..., doc: str | None = ...) -> dict: ...
def cmd(*argv: str, cwd: str | None = ..., env: dict | None = ...) -> dict: ...
def script(path: str, *argv: str, interpreter: str | None = ..., cwd: str | None = ..., env: dict | None = ...) -> dict: ...
def need(name: str, slots: float = ..., scope: str = ..., hold: str = ...) -> dict: ...
def queue_use(name: str, scope: str = ..., slots: int = ..., priority: int = ...) -> dict: ...
def resource(name: str, capacity: float, unit: str | None = ..., scope: str = ...) -> dict: ...
def lock(name: str, scope: str = ...) -> dict: ...
def queue_def(name: str, slots: int, discipline: str = ..., max_pending: int | None = ..., scope: str = ...) -> dict: ...
def rate_limit(name: str, burst: int, refill_per_second: float, scope: str = ...) -> dict: ...
def process_cap(name: str, max_running: int, match: str | None = ..., scope: str = ...) -> dict: ...
def retry(attempts: int = ..., on_exit: list | None = ..., backoff: dict | None = ...) -> dict: ...
def fixed(seconds: float) -> dict: ...
def exp_jitter(min_s: float = ..., max_s: float = ..., jitter: str = ...) -> dict: ...
def Local(id: str, max_parallel_tasks: int = ...) -> dict: ...
def Remote(id: str, endpoint: str | None = ..., transport: dict | None = ..., workspace: dict | None = ..., result: dict | None = ..., runtime: dict | None = ...) -> dict: ...
def ContainerRuntime(image: str, command: list | None = ..., mounts: list | None = ..., env: dict | None = ..., resources: dict | None = ...) -> dict: ...
def LocalOnly(local: dict) -> dict: ...
def RemoteOnly(remote: dict | list[dict]) -> dict: ...
def ByCustomPolicy(policy: object) -> dict: ...
def path(value: str) -> dict: ...
def gitignore() -> dict: ...
def CurrentState(roots: list | None = ..., ignored: list | None = ..., include: list | None = ...) -> dict: ...
"#;
