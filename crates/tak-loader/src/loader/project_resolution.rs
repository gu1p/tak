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
