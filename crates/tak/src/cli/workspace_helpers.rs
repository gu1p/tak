use super::*;

/// Loads a workspace from the current working directory using default loader options.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub(super) fn load_workspace_from_cwd() -> Result<WorkspaceSpec> {
    let cwd = std::env::current_dir().context("failed to resolve current directory")?;
    load_workspace(&cwd, &LoadOptions::default())
        .with_context(|| format!("failed to load Tak workspace from {}", cwd.display()))
}

/// Parses a user-provided CLI label into a fully validated `TaskLabel`.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub(super) fn parse_input_label(
    spec: &WorkspaceSpec,
    value: &str,
    command: &str,
) -> Result<TaskLabel> {
    let trimmed = value.trim();
    if looks_like_path_input(trimmed) {
        bail!(
            "`{trimmed}` is not a valid task label.\n\n{}",
            label_guidance(spec, command)
        );
    }

    let label = parse_label(trimmed, "//").map_err(|e| {
        anyhow!(
            "invalid task label `{trimmed}`: {e}\n\n{}",
            label_guidance(spec, command)
        )
    })?;
    if !spec.tasks.contains_key(&label) {
        bail!(
            "task not found: {}\n\n{}",
            canonical_label(&label),
            label_guidance(spec, command)
        );
    }

    Ok(label)
}

fn looks_like_path_input(value: &str) -> bool {
    value == "."
        || value == ".."
        || value.starts_with("./")
        || value.starts_with("../")
        || (!value.contains(':') && (value.starts_with('/') || value.contains('/')))
}

fn label_guidance(spec: &WorkspaceSpec, command: &str) -> String {
    let mut lines = vec![
        format!("Use `tak {command} <label>` with labels like `//:task` or `//pkg:task`."),
        "Run `tak list` to inspect the current directory workspace.".to_string(),
    ];

    let available = spec
        .tasks
        .keys()
        .take(8)
        .map(canonical_label)
        .collect::<Vec<_>>();
    if !available.is_empty() {
        lines.push("Available targets:".to_string());
        lines.extend(available.into_iter().map(|label| format!("  - {label}")));
    }

    lines.join("\n")
}

pub(super) fn canonical_label(label: &TaskLabel) -> String {
    if label.package == "//" {
        format!("//:{}", label.name)
    } else {
        format!("{}:{}", label.package, label.name)
    }
}
