use super::*;

pub(super) fn unpack_remote_worker_workspace(
    workspace_zip: &[u8],
    execution_root: &Path,
) -> Result<()> {
    let cursor = std::io::Cursor::new(workspace_zip);
    let mut archive = ZipArchive::new(cursor)
        .context("invalid_submit_fields: workspace archive zip decode failed")?;

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .with_context(|| format!("failed to load zip entry index {index}"))?;
        let entry_name = entry.name();
        let normalized = normalize_path_ref("workspace", entry_name)
            .with_context(|| format!("invalid workspace zip entry path `{entry_name}`"))?
            .path;
        if normalized == "." {
            continue;
        }

        if entry
            .unix_mode()
            .is_some_and(|mode| (mode & 0o170000) == 0o120000)
        {
            bail!("invalid workspace zip entry `{entry_name}`: symlink entries are unsupported");
        }

        let output_path = execution_root.join(&normalized);
        if entry.is_dir() || entry_name.ends_with('/') {
            fs::create_dir_all(&output_path).with_context(|| {
                format!(
                    "failed to create workspace directory {}",
                    output_path.display()
                )
            })?;
            continue;
        }

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create workspace parent {}", parent.display())
            })?;
        }
        let mut output_file = fs::File::create(&output_path).with_context(|| {
            format!("failed to create workspace file {}", output_path.display())
        })?;
        std::io::copy(&mut entry, &mut output_file)
            .with_context(|| format!("failed to write workspace file {}", output_path.display()))?;
    }

    Ok(())
}

pub(super) fn snapshot_workspace_files(
    root: &Path,
) -> Result<HashMap<String, WorkspaceFileFingerprint>> {
    let mut fingerprints = HashMap::new();
    if !root.exists() {
        return Ok(fingerprints);
    }
    snapshot_workspace_files_recursive(root, root, &mut fingerprints)?;
    Ok(fingerprints)
}

fn snapshot_workspace_files_recursive(
    root: &Path,
    current: &Path,
    fingerprints: &mut HashMap<String, WorkspaceFileFingerprint>,
) -> Result<()> {
    for entry in fs::read_dir(current)
        .with_context(|| format!("failed to read directory {}", current.display()))?
    {
        let entry =
            entry.with_context(|| format!("failed to iterate directory {}", current.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .with_context(|| format!("failed to inspect path {}", path.display()))?;
        if file_type.is_dir() {
            snapshot_workspace_files_recursive(root, &path, fingerprints)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }

        let relative = path
            .strip_prefix(root)
            .with_context(|| format!("failed to strip workspace prefix for {}", path.display()))?;
        let normalized = relative.to_string_lossy().replace('\\', "/");
        if normalized.is_empty() || normalized == "." {
            continue;
        }

        let bytes =
            fs::read(&path).with_context(|| format!("failed to read file {}", path.display()))?;
        let digest = format!("sha256:{:x}", Sha256::digest(&bytes));
        let size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        fingerprints.insert(normalized, WorkspaceFileFingerprint { digest, size });
    }

    Ok(())
}

pub(super) fn changed_remote_worker_outputs(
    execution_root: &Path,
    before: &HashMap<String, WorkspaceFileFingerprint>,
) -> Result<Vec<RemoteWorkerOutputRecord>> {
    let after = snapshot_workspace_files(execution_root)?;
    let mut outputs = Vec::new();

    for (path, fingerprint) in after {
        let changed = match before.get(&path) {
            Some(previous) => previous != &fingerprint,
            None => true,
        };
        if !changed {
            continue;
        }

        outputs.push(RemoteWorkerOutputRecord {
            path,
            digest: fingerprint.digest,
            size: fingerprint.size,
        });
    }
    outputs.sort_unstable_by(|left, right| left.path.cmp(&right.path));

    Ok(outputs)
}
