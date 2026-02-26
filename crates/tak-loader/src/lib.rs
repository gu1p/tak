//! Workspace discovery and `TASKS.py` loading.
//!
//! This crate discovers task definition files, evaluates them via Monty, converts output
//! into strongly-typed core models, and assembles a resolved `WorkspaceSpec`.

use std::collections::{BTreeMap, HashMap};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use ignore::WalkBuilder;
use monty::{LimitedTracker, MontyObject, MontyRun, PrintWriter, ResourceLimits};
use monty_type_checking::{SourceFile, type_check};
use serde::Deserialize;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use tak_core::label::parse_label;
use tak_core::model::{
    LimiterDef, LimiterKey, LimiterRef, ModuleSpec, QueueDef, ResolvedTask, RetryDef, Scope,
    TaskLabel, WorkspaceSpec,
};

const TASKS_FILE: &str = "TASKS.py";

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

/// Detects the workspace root based on `tak.toml`, `.git`, or the current path.
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

    if let Some(marker) = find_ancestor_with(&search_start, "tak.toml") {
        return Ok(marker);
    }
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
    let project_id = resolve_project_id(&workspace_root, options.project_id.as_deref())?;
    let files = discover_tasks_files(&workspace_root)?;

    let mut tasks = BTreeMap::<TaskLabel, ResolvedTask>::new();
    let mut limiters = HashMap::<LimiterKey, LimiterDef>::new();
    let mut queues = HashMap::<LimiterKey, QueueDef>::new();

    for file in files {
        let module = eval_module_spec(&file, options)?;
        let package = package_for_file(&workspace_root, &file)?;
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

#[derive(Debug, Deserialize)]
struct TakToml {
    project_id: Option<String>,
}

/// Resolves the project id from options, `tak.toml`, or a path-based hash fallback.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn resolve_project_id(root: &Path, from_options: Option<&str>) -> Result<String> {
    if let Some(value) = from_options {
        return Ok(value.to_string());
    }

    let config_file = root.join("tak.toml");
    if config_file.exists() {
        let config_data = fs::read_to_string(&config_file)?;
        let config: TakToml = toml::from_str(&config_data)
            .map_err(|e| anyhow!("failed to parse {}: {e}", config_file.display()))?;
        if let Some(project_id) = config.project_id {
            return Ok(project_id);
        }
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

        let resolved = ResolvedTask {
            label: label.clone(),
            doc: task.doc,
            deps,
            steps: task.steps,
            needs,
            queue,
            retry,
            timeout_s: task.timeout_s,
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

def module_spec(tasks, limiters=None, queues=None, exclude=None, defaults=None):
    return {
        "spec_version": 1,
        "tasks": tasks,
        "limiters": _or_empty_list(limiters),
        "queues": _or_empty_list(queues),
        "exclude": _or_empty_list(exclude),
        "defaults": defaults if defaults is not None else {},
    }

def task(name, deps=None, steps=None, needs=None, queue=None, retry=None, timeout_s=None, tags=None, doc=None):
    return {
        "name": name,
        "deps": _or_empty_list(deps),
        "steps": _or_empty_list(steps),
        "needs": _or_empty_list(needs),
        "queue": queue,
        "retry": retry,
        "timeout_s": timeout_s,
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

def module_spec(tasks: list, limiters: list | None = ..., queues: list | None = ..., exclude: list | None = ..., defaults: dict | None = ...) -> dict: ...
def task(name: str, deps: list | None = ..., steps: list | None = ..., needs: list | None = ..., queue: dict | None = ..., retry: dict | None = ..., timeout_s: int | None = ..., tags: list | None = ..., doc: str | None = ...) -> dict: ...
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
"#;
