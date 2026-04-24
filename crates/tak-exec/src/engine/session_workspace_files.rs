use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use tak_core::model::{CurrentStateSpec, OutputSelectorSpec, build_current_state_manifest};

use super::workspace_collect::{collect_workspace_files, materialize_manifest_files};

pub(super) fn materialize_seed_workspace(
    workspace_root: &Path,
    destination: &Path,
    seed: &CurrentStateSpec,
) -> Result<()> {
    let available = collect_workspace_files(workspace_root, seed)?;
    let manifest = build_current_state_manifest(available, seed);
    materialize_manifest_files(workspace_root, destination, &manifest.entries)
}

pub(super) fn extract_share_paths(
    source_root: &Path,
    store_root: &Path,
    paths: &[OutputSelectorSpec],
) -> Result<()> {
    for selector in paths {
        match selector {
            OutputSelectorSpec::Path(path) => {
                replace_path(source_root, store_root, &path.path)?;
            }
            OutputSelectorSpec::Glob { pattern } => {
                copy_glob_matches(source_root, store_root, pattern)?;
            }
        }
    }
    Ok(())
}

fn replace_path(source_root: &Path, store_root: &Path, relative_path: &str) -> Result<()> {
    let source = source_root.join(relative_path);
    let destination = store_root.join(relative_path);
    remove_existing(&destination)?;
    if source.is_dir() {
        copy_directory_contents(&source, &destination)?;
    } else if source.is_file() {
        copy_file(&source, &destination)?;
    }
    Ok(())
}

fn copy_glob_matches(source_root: &Path, store_root: &Path, pattern: &str) -> Result<()> {
    let mut builder = GitignoreBuilder::new(source_root);
    builder
        .add_line(None, pattern)
        .with_context(|| format!("invalid session share glob `{pattern}`"))?;
    let matcher = builder
        .build()
        .with_context(|| format!("invalid session share glob `{pattern}`"))?;
    copy_matching_files(source_root, source_root, store_root, &matcher)
}

fn copy_matching_files(
    source_root: &Path,
    current: &Path,
    store_root: &Path,
    matcher: &Gitignore,
) -> Result<()> {
    for entry in fs::read_dir(current)
        .with_context(|| format!("failed to read session directory {}", current.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            copy_matching_files(source_root, &path, store_root, matcher)?;
        } else if file_type.is_file() {
            let relative = path.strip_prefix(source_root)?;
            if matcher.matched(relative, false).is_ignore() {
                copy_file(&path, &store_root.join(relative))?;
            }
        }
    }
    Ok(())
}

pub(super) fn copy_directory_contents(source: &Path, destination: &Path) -> Result<()> {
    if !source.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(source)
        .with_context(|| format!("failed to read directory {}", source.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let relative = path.strip_prefix(source)?;
        let destination_path = destination.join(relative);
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            copy_directory_contents(&path, &destination_path)?;
        } else if file_type.is_file() {
            copy_file(&path, &destination_path)?;
        }
    }
    Ok(())
}

fn copy_file(source: &Path, destination: &Path) -> Result<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create session directory {}", parent.display()))?;
    }
    fs::copy(source, destination).with_context(|| {
        format!(
            "failed to copy session file {} -> {}",
            source.display(),
            destination.display()
        )
    })?;
    Ok(())
}

fn remove_existing(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => fs::remove_dir_all(path)?,
        Ok(_) => fs::remove_file(path)?,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(err.into()),
    }
    Ok(())
}
