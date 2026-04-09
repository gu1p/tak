pub fn detect_workspace_root(start: &Path) -> Result<PathBuf> {
    resolve_tasks_file(start)?
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| anyhow!("TASKS.py has no parent: {}", start.display()))
}

/// Discovers the root `TASKS.py` plus any explicit include graph.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub fn discover_tasks_files(
    root: &Path,
    options: &LoadOptions,
) -> Result<Vec<(PathBuf, ModuleSpec)>> {
    let workspace_root = root
        .canonicalize()
        .unwrap_or_else(|_| root.to_path_buf());
    let root_tasks_file = workspace_root.join(TASKS_FILE);
    let mut seen = BTreeSet::new();
    let mut stack = Vec::new();
    let mut modules = Vec::new();

    collect_tasks_files(
        &root_tasks_file,
        &workspace_root,
        options,
        &mut seen,
        &mut stack,
        &mut modules,
    )?;
    Ok(modules)
}

fn resolve_tasks_file(start: &Path) -> Result<PathBuf> {
    let candidate = if start.is_file() {
        start.to_path_buf()
    } else {
        start.join(TASKS_FILE)
    };

    if candidate
        .file_name()
        .is_none_or(|name| name != TASKS_FILE)
    {
        bail!(
            "expected `{TASKS_FILE}` in the current directory, got {}",
            candidate.display()
        );
    }
    if !candidate.is_file() {
        let directory = candidate
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| start.to_path_buf());
        bail!(
            "no `{TASKS_FILE}` found in current directory {}\nRun Tak from a directory that contains `{TASKS_FILE}`.",
            directory.display()
        );
    }

    candidate
        .canonicalize()
        .with_context(|| format!("failed to canonicalize {}", candidate.display()))
}

fn collect_tasks_files(
    tasks_file: &Path,
    workspace_root: &Path,
    options: &LoadOptions,
    seen: &mut BTreeSet<PathBuf>,
    stack: &mut Vec<PathBuf>,
    modules: &mut Vec<(PathBuf, ModuleSpec)>,
) -> Result<()> {
    if let Some(index) = stack.iter().position(|path| path == tasks_file) {
        let cycle = stack[index..]
            .iter()
            .chain(std::iter::once(&tasks_file.to_path_buf()))
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(" -> ");
        bail!("include cycle detected: {cycle}");
    }
    if !seen.insert(tasks_file.to_path_buf()) {
        return Ok(());
    }

    stack.push(tasks_file.to_path_buf());
    let module = eval_module_spec(tasks_file, options)?;
    let includes = module.includes.clone();
    modules.push((tasks_file.to_path_buf(), module));

    for include in includes {
        let include_tasks = resolve_include_tasks_file(tasks_file, workspace_root, &include)?;
        collect_tasks_files(&include_tasks, workspace_root, options, seen, stack, modules)?;
    }

    stack.pop();
    Ok(())
}

fn resolve_include_tasks_file(
    current_tasks_file: &Path,
    workspace_root: &Path,
    include: &PathInputDef,
) -> Result<PathBuf> {
    let raw = match include {
        PathInputDef::Path { value } => value,
    };
    let base_dir = current_tasks_file
        .parent()
        .ok_or_else(|| anyhow!("TASKS.py has no parent: {}", current_tasks_file.display()))?;
    let candidate = base_dir.join(raw);
    let candidate = if candidate.is_dir() {
        candidate.join(TASKS_FILE)
    } else {
        candidate
    };
    if !candidate.is_file() {
        bail!(
            "include `{raw}` from {} does not resolve to a `{TASKS_FILE}` file",
            current_tasks_file.display()
        );
    }

    let canonical = candidate
        .canonicalize()
        .with_context(|| format!("failed to canonicalize include {}", candidate.display()))?;
    if !canonical.starts_with(workspace_root) {
        bail!(
            "include `{raw}` from {} escapes workspace root {}",
            current_tasks_file.display(),
            workspace_root.display()
        );
    }
    Ok(canonical)
}
