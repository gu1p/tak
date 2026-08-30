use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, anyhow, bail};
use tak_core::v2::{AuthoredModule, AuthoredTask};

pub(super) fn selected_tasks<'a>(
    module: &'a AuthoredModule,
    labels: &[String],
) -> Result<Vec<&'a AuthoredTask>> {
    let mut known = BTreeMap::new();
    for task in &module.tasks {
        let label = canonical(&task.name)?;
        if known.insert(label.clone(), task).is_some() {
            bail!("duplicate task {label}");
        }
    }
    let mut ordered = Vec::new();
    let mut visited = BTreeSet::new();
    let mut visiting = BTreeSet::new();
    for label in labels {
        visit(
            &canonical(label)?,
            &known,
            &mut visiting,
            &mut visited,
            &mut ordered,
        )?;
    }
    Ok(ordered)
}

fn visit<'a>(
    label: &str,
    known: &BTreeMap<String, &'a AuthoredTask>,
    visiting: &mut BTreeSet<String>,
    visited: &mut BTreeSet<String>,
    ordered: &mut Vec<&'a AuthoredTask>,
) -> Result<()> {
    if visited.contains(label) {
        return Ok(());
    }
    if !visiting.insert(label.to_owned()) {
        bail!("task graph contains a cycle at {label}");
    }
    let task = known
        .get(label)
        .ok_or_else(|| anyhow!("task not found: {label}"))?;
    for dependency in &task.deps {
        visit(&canonical(dependency)?, known, visiting, visited, ordered)?;
    }
    visiting.remove(label);
    visited.insert(label.to_owned());
    ordered.push(*task);
    Ok(())
}

pub(super) fn canonical(value: &str) -> Result<String> {
    let name = value
        .strip_prefix("//:")
        .or_else(|| value.strip_prefix(':'))
        .unwrap_or(value);
    if name.is_empty() || name.contains('/') || name.contains(':') {
        bail!("v2 root task label is invalid: {value}");
    }
    Ok(format!("//:{name}"))
}
