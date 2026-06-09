use std::fs;
use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use tak_core::model::{
    CurrentStateOrigin, CurrentStateSpec, IgnoreSourceSpec, PathAnchor, PathRef,
    build_current_state_manifest,
};

use super::workspace_sync::normalize_filesystem_relative_path;

/// Identity of the workspace a remote task would stage: a content hash (the per-job
/// upload-cache key) plus the manifest hash (recorded on the task result).
pub(crate) struct WorkspaceUploadIdentity {
    /// Deterministic, content-sensitive hash (paths + file bytes) — the upload-cache key, so
    /// byte-identical workspaces are uploaded to a node only once.
    pub(crate) content_hash: String,
    /// Paths-only manifest hash, surfaced as `TaskRunResult.context_manifest_hash`. Stable
    /// regardless of whether this task staged or reused a cached upload.
    pub(crate) manifest_hash: String,
}

/// Computes the [`WorkspaceUploadIdentity`] of the workspace a remote task would stage.
///
/// It hashes the SAME logical file set that staging zips — `collect_workspace_files` +
/// `build_current_state_manifest` (already sorted + deduped) — folding each entry's anchor,
/// path, and a SHA-256 of its bytes into the content hash. Deliberately NOT the staged zip's
/// sha256 (the `zip` crate stamps wall-clock mtimes, so it is non-deterministic) nor
/// `manifest.hash` (paths only — different file contents at the same paths collide).
///
/// `state` must be the same `CurrentState` staging uses (e.g. from the session-context-resolved
/// task), so the hashed set matches exactly what would be uploaded.
pub(crate) fn workspace_upload_identity(
    workspace_root: &Path,
    state: &CurrentStateSpec,
) -> Result<WorkspaceUploadIdentity> {
    let available = collect_workspace_files(workspace_root, state)?;
    let manifest = build_current_state_manifest(available, state);
    let mut hasher = Sha256::new();
    for entry in &manifest.entries {
        if entry.path == "." {
            continue;
        }
        let source = workspace_root.join(&entry.path);
        if !source.is_file() {
            continue;
        }
        let anchor = content_hash_anchor_tag(&entry.anchor);
        hasher.update((anchor.len() as u64).to_be_bytes());
        hasher.update(anchor.as_bytes());
        hasher.update((entry.path.len() as u64).to_be_bytes());
        hasher.update(entry.path.as_bytes());
        let file_digest = hash_file_contents(&source)?;
        hasher.update(file_digest);
    }
    Ok(WorkspaceUploadIdentity {
        content_hash: format!("{:x}", hasher.finalize()),
        manifest_hash: manifest.hash,
    })
}

fn content_hash_anchor_tag(anchor: &PathAnchor) -> String {
    match anchor {
        PathAnchor::Workspace => "workspace".to_string(),
        PathAnchor::Package => "package".to_string(),
        PathAnchor::Repo(name) => format!("repo:{name}"),
    }
}

fn hash_file_contents(path: &Path) -> Result<[u8; 32]> {
    let mut file =
        fs::File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .with_context(|| format!("failed to read {}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().into())
}

pub(crate) fn collect_workspace_files(
    workspace_root: &Path,
    state: &CurrentStateSpec,
) -> Result<Vec<PathRef>> {
    let mut files = if uses_workspace_gitignore(state) {
        collect_workspace_files_with_gitignore(workspace_root)?
    } else {
        let mut files = Vec::new();
        collect_workspace_files_recursive(workspace_root, workspace_root, &mut files)?;
        files
    };
    collect_explicit_include_files(workspace_root, &state.include, &mut files)?;
    Ok(files)
}

fn uses_workspace_gitignore(state: &CurrentStateSpec) -> bool {
    state.origin == CurrentStateOrigin::ImplicitDefault || uses_gitignore_ignore_source(state)
}

fn uses_gitignore_ignore_source(state: &CurrentStateSpec) -> bool {
    state
        .ignored
        .iter()
        .any(|source| matches!(source, IgnoreSourceSpec::GitIgnore))
}

fn collect_workspace_files_with_gitignore(workspace_root: &Path) -> Result<Vec<PathRef>> {
    let mut builder = ignore::WalkBuilder::new(workspace_root);
    builder
        .hidden(false)
        .ignore(false)
        .git_global(false)
        .git_ignore(true)
        .git_exclude(false)
        .parents(true)
        .require_git(false);

    let mut files = Vec::new();
    for entry in builder.build() {
        let entry = entry.with_context(|| {
            format!(
                "failed to read workspace entry during gitignore-aware scan under {}",
                workspace_root.display()
            )
        })?;
        let Some(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_file() {
            continue;
        }

        let path = entry.path();
        push_workspace_file(workspace_root, path, &mut files)?;
    }

    Ok(files)
}

fn collect_workspace_files_recursive(
    workspace_root: &Path,
    current_dir: &Path,
    files: &mut Vec<PathRef>,
) -> Result<()> {
    for entry in fs::read_dir(current_dir).with_context(|| {
        format!(
            "failed to read workspace directory {}",
            current_dir.display()
        )
    })? {
        let entry = entry.with_context(|| {
            format!(
                "failed to read workspace entry under {}",
                current_dir.display()
            )
        })?;
        let path = entry.path();
        let file_type = entry.file_type().with_context(|| {
            format!(
                "failed to read file type for workspace entry {}",
                path.display()
            )
        })?;

        if file_type.is_dir() {
            collect_workspace_files_recursive(workspace_root, &path, files)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }

        push_workspace_file(workspace_root, &path, files)?;
    }

    Ok(())
}

fn push_workspace_file(workspace_root: &Path, path: &Path, files: &mut Vec<PathRef>) -> Result<()> {
    let relative = path.strip_prefix(workspace_root).with_context(|| {
        format!(
            "failed to compute relative path for workspace file {}",
            path.display()
        )
    })?;
    files.push(PathRef {
        anchor: PathAnchor::Workspace,
        path: normalize_filesystem_relative_path(relative),
    });
    Ok(())
}

fn collect_explicit_include_files(
    workspace_root: &Path,
    includes: &[PathRef],
    files: &mut Vec<PathRef>,
) -> Result<()> {
    for include in includes {
        if include.anchor != PathAnchor::Workspace {
            bail!(
                "unsupported non-workspace context include during staging: {:?}",
                include.anchor
            );
        }
        if include.path == "." {
            collect_workspace_files_recursive(workspace_root, workspace_root, files)?;
            continue;
        }

        let include_path = workspace_root.join(&include.path);
        if include_path.is_dir() {
            collect_workspace_files_recursive(workspace_root, &include_path, files)?;
            continue;
        }
        if !include_path.is_file() {
            continue;
        }

        files.push(PathRef {
            anchor: PathAnchor::Workspace,
            path: include.path.clone(),
        });
    }

    Ok(())
}

pub(crate) fn materialize_manifest_files(
    workspace_root: &Path,
    staged_root: &Path,
    entries: &[PathRef],
) -> Result<()> {
    for entry in entries {
        if entry.anchor != PathAnchor::Workspace {
            bail!(
                "unsupported non-workspace context manifest anchor during staging: {:?}",
                entry.anchor
            );
        }
        if entry.path == "." {
            continue;
        }

        let source = workspace_root.join(&entry.path);
        if !source.is_file() {
            continue;
        }
        let destination = staged_root.join(&entry.path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create staged directory {}",
                    parent.to_string_lossy()
                )
            })?;
        }
        fs::copy(&source, &destination).with_context(|| {
            format!(
                "failed to stage context file {} -> {}",
                source.display(),
                destination.display()
            )
        })?;
    }

    Ok(())
}

#[cfg(test)]
mod content_hash_tests {
    use super::workspace_upload_identity;
    use std::fs;
    use tak_core::model::CurrentStateSpec;

    fn seed_tree(root: &std::path::Path, payload: &[u8]) {
        fs::write(root.join("top.txt"), payload).expect("top file");
        fs::create_dir_all(root.join("nested")).expect("nested dir");
        fs::write(root.join("nested/inner.txt"), b"inner").expect("inner file");
    }

    fn content_hash(root: &std::path::Path, state: &CurrentStateSpec) -> String {
        workspace_upload_identity(root, state)
            .expect("upload identity")
            .content_hash
    }

    #[test]
    fn identical_content_hashes_identically_and_is_content_sensitive() {
        let state = CurrentStateSpec::default();

        let a = tempfile::tempdir().expect("tempdir a");
        seed_tree(a.path(), b"hello");
        let hash_a = content_hash(a.path(), &state);

        // Deterministic across repeated calls (no wall-clock / ordering dependence).
        assert_eq!(hash_a, content_hash(a.path(), &state));

        // Byte-identical content in a different directory hashes the same.
        let b = tempfile::tempdir().expect("tempdir b");
        seed_tree(b.path(), b"hello");
        assert_eq!(hash_a, content_hash(b.path(), &state));

        // Changing a file's contents (same paths) must change the hash — this is what a
        // paths-only manifest hash would miss.
        let c = tempfile::tempdir().expect("tempdir c");
        seed_tree(c.path(), b"HELLO");
        assert_ne!(hash_a, content_hash(c.path(), &state));
    }
}
