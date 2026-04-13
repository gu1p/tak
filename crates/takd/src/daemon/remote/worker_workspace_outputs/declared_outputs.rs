use super::*;

pub(in crate::daemon::remote) fn collect_declared_remote_worker_outputs(
    execution_root: &Path,
    selectors: &[OutputSelectorSpec],
    require_matches: bool,
) -> Result<Vec<RemoteWorkerOutputRecord>> {
    if selectors.is_empty() {
        return Ok(Vec::new());
    }

    let mut outputs = HashMap::<String, RemoteWorkerOutputRecord>::new();
    for selector in selectors {
        let matched = match selector {
            OutputSelectorSpec::Path(path) => collect_declared_output_path(
                execution_root,
                &path.path,
                &mut outputs,
                require_matches,
            )?,
            OutputSelectorSpec::Glob { pattern } => {
                collect_declared_output_glob(execution_root, pattern, &mut outputs)?
            }
        };
        if require_matches && matched == 0 {
            match selector {
                OutputSelectorSpec::Path(path) => {
                    bail!("declared output path `{}` matched no files", path.path)
                }
                OutputSelectorSpec::Glob { pattern } => {
                    bail!("declared output glob `{pattern}` matched no files")
                }
            }
        }
    }

    let mut outputs = outputs.into_values().collect::<Vec<_>>();
    outputs.sort_unstable_by(|left, right| left.path.cmp(&right.path));
    Ok(outputs)
}

fn collect_declared_output_path(
    execution_root: &Path,
    relative_path: &str,
    outputs: &mut HashMap<String, RemoteWorkerOutputRecord>,
    require_matches: bool,
) -> Result<usize> {
    let candidate = execution_root.join(relative_path);
    let metadata = fs::metadata(&candidate).with_context(|| {
        format!("declared output path `{relative_path}` was not created in remote workspace")
    });
    let metadata = match metadata {
        Ok(metadata) => metadata,
        Err(error) if !require_matches && is_missing_declared_output(&error) => return Ok(0),
        Err(error) => return Err(error),
    };

    if metadata.is_file() {
        insert_output_file(execution_root, &candidate, outputs)?;
        return Ok(1);
    }
    if metadata.is_dir() {
        return collect_declared_output_directory(execution_root, &candidate, outputs);
    }

    bail!("declared output path `{relative_path}` must resolve to a file or directory");
}

fn is_missing_declared_output(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<std::io::Error>()
        .is_some_and(|io_error| io_error.kind() == std::io::ErrorKind::NotFound)
}

fn collect_declared_output_directory(
    execution_root: &Path,
    current: &Path,
    outputs: &mut HashMap<String, RemoteWorkerOutputRecord>,
) -> Result<usize> {
    let mut matched = 0_usize;
    for entry in fs::read_dir(current).with_context(|| {
        format!(
            "failed to read declared output directory {}",
            current.display()
        )
    })? {
        let entry = entry
            .with_context(|| format!("failed to iterate declared output {}", current.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .with_context(|| format!("failed to inspect declared output {}", path.display()))?;
        if file_type.is_dir() {
            matched += collect_declared_output_directory(execution_root, &path, outputs)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        insert_output_file(execution_root, &path, outputs)?;
        matched += 1;
    }
    Ok(matched)
}

fn collect_declared_output_glob(
    execution_root: &Path,
    pattern: &str,
    outputs: &mut HashMap<String, RemoteWorkerOutputRecord>,
) -> Result<usize> {
    let mut builder = GitignoreBuilder::new(execution_root);
    builder
        .add_line(None, pattern)
        .with_context(|| format!("invalid declared output glob `{pattern}`"))?;
    let matcher = builder
        .build()
        .with_context(|| format!("invalid declared output glob `{pattern}`"))?;
    collect_declared_glob_matches(execution_root, execution_root, &matcher, outputs)
}

fn collect_declared_glob_matches(
    execution_root: &Path,
    current: &Path,
    matcher: &Gitignore,
    outputs: &mut HashMap<String, RemoteWorkerOutputRecord>,
) -> Result<usize> {
    let mut matched = 0_usize;
    for entry in fs::read_dir(current).with_context(|| {
        format!(
            "failed to read remote workspace directory {}",
            current.display()
        )
    })? {
        let entry = entry
            .with_context(|| format!("failed to iterate remote workspace {}", current.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .with_context(|| format!("failed to inspect remote workspace {}", path.display()))?;
        if file_type.is_dir() {
            matched += collect_declared_glob_matches(execution_root, &path, matcher, outputs)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }

        let relative = path.strip_prefix(execution_root).with_context(|| {
            format!(
                "failed to strip remote workspace prefix for declared output {}",
                path.display()
            )
        })?;
        if matcher.matched(relative, false).is_ignore() {
            insert_output_file(execution_root, &path, outputs)?;
            matched += 1;
        }
    }
    Ok(matched)
}

fn insert_output_file(
    execution_root: &Path,
    path: &Path,
    outputs: &mut HashMap<String, RemoteWorkerOutputRecord>,
) -> Result<()> {
    let relative = path.strip_prefix(execution_root).with_context(|| {
        format!(
            "failed to strip remote workspace prefix for declared output {}",
            path.display()
        )
    })?;
    let normalized = relative.to_string_lossy().replace('\\', "/");
    if normalized.is_empty() || normalized == "." {
        bail!(
            "declared output {} resolved to an invalid path",
            path.display()
        );
    }

    let bytes = fs::read(path)
        .with_context(|| format!("failed to read declared output {}", path.display()))?;
    let digest = format!("sha256:{:x}", Sha256::digest(&bytes));
    let size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    outputs.insert(
        normalized.clone(),
        RemoteWorkerOutputRecord {
            path: normalized,
            digest,
            size,
        },
    );
    Ok(())
}
