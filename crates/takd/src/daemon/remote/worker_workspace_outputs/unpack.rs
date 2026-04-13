use super::*;

pub(in crate::daemon::remote) fn unpack_remote_worker_workspace(
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
