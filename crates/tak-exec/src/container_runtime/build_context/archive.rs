use super::*;

use std::fs;

pub(super) fn build_context_archive(build_context_root: &Path) -> Result<Vec<u8>> {
    let mut files = Vec::new();
    collect_build_context_files(build_context_root, build_context_root, &mut files)?;
    files.sort_by(|left, right| left.0.cmp(&right.0));

    let mut archive = Vec::new();
    {
        let mut builder = tar_builder(&mut archive);
        for (relative, absolute, mode) in files {
            append_tar_entry(&mut builder, &relative, &absolute, mode)?;
        }
        builder
            .finish()
            .context("failed to finalize build context archive")?;
    }
    Ok(archive)
}

pub(super) fn collect_build_context_files(
    build_context_root: &Path,
    current_dir: &Path,
    files: &mut Vec<(String, PathBuf, u32)>,
) -> Result<()> {
    for entry in fs::read_dir(current_dir).with_context(|| {
        format!(
            "failed to read build context directory {}",
            current_dir.display()
        )
    })? {
        let entry = entry.with_context(|| {
            format!(
                "failed to read build context entry under {}",
                current_dir.display()
            )
        })?;
        let path = entry.path();
        let file_type = entry.file_type().with_context(|| {
            format!(
                "failed to read build context file type for {}",
                path.display()
            )
        })?;

        if file_type.is_dir() {
            collect_build_context_files(build_context_root, &path, files)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }

        let relative = path.strip_prefix(build_context_root).with_context(|| {
            format!(
                "failed to compute build context relative path for {}",
                path.display()
            )
        })?;
        let metadata = entry.metadata().with_context(|| {
            format!(
                "failed to read build context metadata for {}",
                path.display()
            )
        })?;
        files.push((
            normalize_archive_path(relative),
            path,
            archive_mode(&metadata),
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn archive_mode(metadata: &fs::Metadata) -> u32 {
    use std::os::unix::fs::PermissionsExt;

    metadata.permissions().mode() & 0o7777
}

#[cfg(not(unix))]
fn archive_mode(_metadata: &fs::Metadata) -> u32 {
    0o644
}

pub(super) fn normalize_archive_path(path: &Path) -> String {
    let mut normalized = String::new();
    for component in path.components() {
        if !normalized.is_empty() {
            normalized.push('/');
        }
        normalized.push_str(&component.as_os_str().to_string_lossy());
    }
    if normalized.is_empty() {
        ".".to_string()
    } else {
        normalized
    }
}
