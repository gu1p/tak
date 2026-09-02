use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use tak_core::v2::{AuthoredModule, OutputSelector};

use super::{LoadOptions, TASKS_FILE, v2_module_eval};

mod normalize;
mod normalize_context;

pub(super) fn evaluate(
    workspace_root: &Path,
    tasks_file: &Path,
    options: &LoadOptions,
) -> Result<AuthoredModule> {
    let mut collector = Collector {
        workspace_root,
        options,
        seen: BTreeSet::new(),
        stack: Vec::new(),
        modules: Vec::new(),
    };
    collector.collect(tasks_file)?;
    merge(workspace_root, collector.modules)
}

struct Collector<'a> {
    workspace_root: &'a Path,
    options: &'a LoadOptions,
    seen: BTreeSet<PathBuf>,
    stack: Vec<PathBuf>,
    modules: Vec<(PathBuf, AuthoredModule)>,
}

impl Collector<'_> {
    fn collect(&mut self, tasks_file: &Path) -> Result<()> {
        if let Some(index) = self.stack.iter().position(|path| path == tasks_file) {
            let cycle = self.stack[index..]
                .iter()
                .map(PathBuf::as_path)
                .chain(std::iter::once(tasks_file))
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(" -> ");
            bail!("include cycle detected: {cycle}");
        }
        if !self.seen.insert(tasks_file.to_path_buf()) {
            return Ok(());
        }
        self.stack.push(tasks_file.to_path_buf());
        let module = v2_module_eval::evaluate(tasks_file, self.options)?;
        let includes = module.includes.clone();
        self.modules.push((tasks_file.to_path_buf(), module));
        for include in includes {
            let child = resolve_include(tasks_file, self.workspace_root, &include)?;
            self.collect(&child)?;
        }
        self.stack.pop();
        Ok(())
    }
}

fn resolve_include(current: &Path, root: &Path, include: &OutputSelector) -> Result<PathBuf> {
    let OutputSelector::Path { value } = include else {
        bail!("module_spec(includes=...) accepts path(...) entries, not glob(...)")
    };
    let parent = current
        .parent()
        .ok_or_else(|| anyhow!("TASKS.py has no parent: {}", current.display()))?;
    let candidate = parent.join(value);
    let candidate = if candidate.is_dir() {
        candidate.join(TASKS_FILE)
    } else {
        candidate
    };
    if !candidate.is_file() {
        bail!(
            "include `{value}` from {} does not resolve to a `{TASKS_FILE}` file",
            current.display()
        );
    }
    let canonical = candidate
        .canonicalize()
        .with_context(|| format!("failed to canonicalize include {}", candidate.display()))?;
    if !canonical.starts_with(root) {
        bail!(
            "include `{value}` from {} escapes workspace root {}",
            current.display(),
            root.display()
        );
    }
    Ok(canonical)
}

fn merge(root: &Path, modules: Vec<(PathBuf, AuthoredModule)>) -> Result<AuthoredModule> {
    let mut result = AuthoredModule::default();
    let mut labels = BTreeSet::new();
    for (index, (path, mut module)) in modules.into_iter().enumerate() {
        let package = normalize::package(root, &path)?;
        normalize::module(&mut module, &package, index == 0)?;
        for task in &mut module.tasks {
            task.name = normalize::label(&task.name, &package)?;
            task.deps = task
                .deps
                .iter()
                .map(|dependency| normalize::label(dependency, &package))
                .collect::<Result<Vec<_>>>()?;
            if !labels.insert(task.name.clone()) {
                bail!("duplicate task {}", task.name);
            }
        }
        if index == 0 {
            result.project_id = module.project_id.take();
            result.defaults = std::mem::take(&mut module.defaults);
        } else if module.project_id.is_some() && module.project_id != result.project_id {
            bail!(
                "included module {} declares a conflicting project_id",
                path.display()
            );
        }
        result.tasks.extend(module.tasks);
        result
            .limiter_definitions
            .extend(module.limiter_definitions);
        result.queue_definitions.extend(module.queue_definitions);
        result.exclude.extend(module.exclude);
    }
    Ok(result)
}
