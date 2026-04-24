use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use sha2::{Digest, Sha256};
use tak_core::model::OutputSelectorSpec;

use super::{SyncedOutput, workspace_sync::normalize_filesystem_relative_path};

pub(crate) fn collect_workspace_outputs(
    root: &Path,
    selectors: &[OutputSelectorSpec],
    require_matches: bool,
) -> Result<Vec<SyncedOutput>> {
    let mut outputs = HashMap::<String, SyncedOutput>::new();
    for selector in selectors {
        let matched = match selector {
            OutputSelectorSpec::Path(path) => {
                collect_output_path(root, &path.path, &mut outputs, require_matches)?
            }
            OutputSelectorSpec::Glob { pattern } => {
                collect_output_glob(root, pattern, &mut outputs)?
            }
        };
        if require_matches && matched == 0 {
            bail!("declared output matched no files");
        }
    }

    let mut values = outputs.into_values().collect::<Vec<_>>();
    values.sort_unstable_by(|left, right| left.path.cmp(&right.path));
    Ok(values)
}

fn collect_output_path(
    root: &Path,
    relative_path: &str,
    outputs: &mut HashMap<String, SyncedOutput>,
    require_matches: bool,
) -> Result<usize> {
    let candidate = root.join(relative_path);
    let metadata = match fs::metadata(&candidate) {
        Ok(metadata) => metadata,
        Err(err) if !require_matches && err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(0);
        }
        Err(err) => {
            return Err(err).with_context(|| {
                format!("declared output path `{relative_path}` was not created")
            });
        }
    };
    if metadata.is_file() {
        insert_output(root, &candidate, outputs)?;
        return Ok(1);
    }
    if metadata.is_dir() {
        return collect_output_directory(root, &candidate, outputs);
    }
    bail!("declared output path `{relative_path}` must resolve to a file or directory")
}

fn collect_output_directory(
    root: &Path,
    current: &Path,
    outputs: &mut HashMap<String, SyncedOutput>,
) -> Result<usize> {
    let mut matched = 0;
    for entry in fs::read_dir(current)
        .with_context(|| format!("failed to read output directory {}", current.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            matched += collect_output_directory(root, &path, outputs)?;
            continue;
        }
        if file_type.is_file() {
            insert_output(root, &path, outputs)?;
            matched += 1;
        }
    }
    Ok(matched)
}

fn collect_output_glob(
    root: &Path,
    pattern: &str,
    outputs: &mut HashMap<String, SyncedOutput>,
) -> Result<usize> {
    let mut builder = GitignoreBuilder::new(root);
    builder
        .add_line(None, pattern)
        .with_context(|| format!("invalid output glob `{pattern}`"))?;
    let matcher = builder
        .build()
        .with_context(|| format!("invalid output glob `{pattern}`"))?;
    collect_glob_matches(root, root, &matcher, outputs)
}

fn collect_glob_matches(
    root: &Path,
    current: &Path,
    matcher: &Gitignore,
    outputs: &mut HashMap<String, SyncedOutput>,
) -> Result<usize> {
    let mut matched = 0;
    for entry in fs::read_dir(current)
        .with_context(|| format!("failed to read output directory {}", current.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            matched += collect_glob_matches(root, &path, matcher, outputs)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        let relative = path.strip_prefix(root)?;
        if matcher.matched(relative, false).is_ignore() {
            insert_output(root, &path, outputs)?;
            matched += 1;
        }
    }
    Ok(matched)
}

fn insert_output(
    root: &Path,
    path: &Path,
    outputs: &mut HashMap<String, SyncedOutput>,
) -> Result<()> {
    let relative = path.strip_prefix(root)?;
    let normalized = normalize_filesystem_relative_path(relative);
    let bytes =
        fs::read(path).with_context(|| format!("failed to read output {}", path.display()))?;
    outputs.insert(
        normalized.clone(),
        SyncedOutput {
            path: normalized,
            digest: format!("sha256:{:x}", Sha256::digest(&bytes)),
            size_bytes: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        },
    );
    Ok(())
}
