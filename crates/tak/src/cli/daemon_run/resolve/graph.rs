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
        let trimmed = label.trim();
        if crate::cli::workspace_helpers::looks_like_path_input(trimmed) {
            let available = known.keys().take(8).cloned().collect::<Vec<_>>();
            bail!(
                "`{trimmed}` is not a valid task label.\n\n{}",
                crate::cli::workspace_helpers::label_guidance_for("run", &available)
            );
        }
        visit(
            &canonical(trimmed)?,
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
    let label = tak_core::label::parse_label(value, "//")
        .map_err(|error| anyhow!("invalid v2 task label `{value}`: {error}"))?;
    Ok(if label.package == "//" {
        format!("//:{}", label.name)
    } else {
        format!("{}:{}", label.package, label.name)
    })
}
