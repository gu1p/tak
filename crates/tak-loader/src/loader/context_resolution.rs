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
        origin: tak_core::model::CurrentStateOrigin::Explicit,
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
