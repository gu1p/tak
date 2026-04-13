fn resolve_output_selectors(
    outputs: Vec<OutputSelectorDef>,
    package: &str,
) -> Result<Vec<OutputSelectorSpec>> {
    outputs
        .into_iter()
        .map(|selector| resolve_output_selector(selector, package))
        .collect()
}

fn resolve_output_selector(selector: OutputSelectorDef, package: &str) -> Result<OutputSelectorSpec> {
    match selector {
        OutputSelectorDef::Path { value } => Ok(OutputSelectorSpec::Path(
            resolve_context_path(PathInputDef::Path { value }, package)
                .map_err(|err| anyhow!("invalid output path: {err}"))?,
        )),
        OutputSelectorDef::Glob { value } => Ok(OutputSelectorSpec::Glob {
            pattern: resolve_output_glob(&value, package)?,
        }),
    }
}

fn resolve_output_glob(raw: &str, package: &str) -> Result<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        bail!("output glob cannot be empty");
    }
    if raw.starts_with('@') {
        bail!("output repo anchors are not supported yet in V1: {raw}");
    }

    let joined = if let Some(workspace_relative) = raw.strip_prefix("//") {
        workspace_relative.to_string()
    } else {
        let package_relative = package.trim_start_matches("//");
        if package_relative.is_empty() {
            raw.to_string()
        } else {
            format!("{package_relative}/{raw}")
        }
    };

    let mut parts = Vec::<String>::new();
    for segment in joined.replace('\\', "/").split('/') {
        match segment {
            "" | "." => continue,
            ".." => {
                if parts.pop().is_none() {
                    bail!("output glob escapes workspace: {raw}");
                }
            }
            value => parts.push(value.to_string()),
        }
    }
    if parts.is_empty() {
        bail!("output glob cannot be empty");
    }

    let normalized = parts.join("/");
    if normalized.starts_with('!') {
        bail!("output glob cannot be negated: {normalized}");
    }

    let mut builder = GitignoreBuilder::new(".");
    builder
        .add_line(None, &normalized)
        .map_err(|err| anyhow!("invalid output glob `{normalized}`: {err}"))?;
    Ok(normalized)
}
