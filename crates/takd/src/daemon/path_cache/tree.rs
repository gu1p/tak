use std::fs;
use std::path::{Component, Path};

use anyhow::{Context, Result, bail};
use ignore::gitignore::GitignoreBuilder;
use tak_core::v2::{OutputSelector, WorkspaceEntry};

pub(super) fn validate_selectors(selectors: &[OutputSelector]) -> Result<()> {
    for selector in selectors {
        let value = match selector {
            OutputSelector::Path { value } | OutputSelector::Glob { value } => value,
        };
        if value.is_empty()
            || value.contains('\\')
            || Path::new(value).components().any(|part| {
                matches!(
                    part,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            bail!("session cache selector must stay inside the workspace");
        }
    }
    Ok(())
}

pub(super) fn capture(
    source: &Path,
    destination: &Path,
    selectors: &[OutputSelector],
) -> Result<()> {
    fs::create_dir_all(destination)?;
    for selector in selectors {
        match selector {
            OutputSelector::Path { value } => {
                copy_optional(source, &source.join(value), destination)?;
            }
            OutputSelector::Glob { value } => copy_glob(source, destination, value)?,
        }
    }
    Ok(())
}

pub(super) fn overlay(source: &Path, destination: &Path) -> Result<()> {
    if !source.try_exists()? {
        return Ok(());
    }
    for child in children(source)? {
        copy_entry(source, &child, destination)?;
    }
    Ok(())
}

fn copy_glob(source: &Path, destination: &Path, pattern: &str) -> Result<()> {
    let mut builder = GitignoreBuilder::new(source);
    builder.add_line(None, pattern)?;
    let matcher = builder.build()?;
    for path in descendants(source)? {
        let metadata = fs::symlink_metadata(&path)?;
        if matcher
            .matched(path.strip_prefix(source)?, metadata.is_dir())
            .is_ignore()
        {
            copy_entry(source, &path, destination)?;
        }
    }
    Ok(())
}

fn copy_optional(root: &Path, source: &Path, destination: &Path) -> Result<()> {
    match fs::symlink_metadata(source) {
        Ok(_) => copy_entry(root, source, destination),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn copy_entry(root: &Path, source: &Path, destination: &Path) -> Result<()> {
    let relative = source.strip_prefix(root)?;
    let target = destination.join(relative);
    remove(&target)?;
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    let metadata = fs::symlink_metadata(source)?;
    if metadata.file_type().is_symlink() {
        copy_symlink(root, source, &target)?;
    } else if metadata.is_dir() {
        fs::create_dir_all(&target)?;
        for child in children(source)? {
            copy_entry(root, &child, destination)?;
        }
    } else if metadata.is_file() {
        crate::daemon::workspace_layer::private_copy_shallow(source, &target)?;
    } else {
        bail!("session cache contains an unsupported entry");
    }
    Ok(())
}

fn copy_symlink(root: &Path, source: &Path, target: &Path) -> Result<()> {
    let relative = source.strip_prefix(root)?.to_string_lossy();
    let link = fs::read_link(source)?;
    let link = link
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("session cache symlink target is not UTF-8"))?;
    WorkspaceEntry::symlink(relative, link)?;
    #[cfg(unix)]
    std::os::unix::fs::symlink(link, target)?;
    #[cfg(not(unix))]
    bail!("session cache symlinks are unsupported on this platform");
    Ok(())
}

fn descendants(root: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut result = Vec::new();
    let mut pending = children(root)?;
    while let Some(path) = pending.pop() {
        if fs::symlink_metadata(&path)?.is_dir() {
            pending.extend(children(&path)?);
        }
        result.push(path);
    }
    result.sort();
    Ok(result)
}

fn children(path: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut result = fs::read_dir(path)
        .with_context(|| format!("read session cache directory {}", path.display()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    result.sort_by(|left, right| right.cmp(left));
    Ok(result)
}

pub(super) fn remove(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
            fs::remove_dir_all(path)?
        }
        Ok(_) => fs::remove_file(path)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    Ok(())
}
