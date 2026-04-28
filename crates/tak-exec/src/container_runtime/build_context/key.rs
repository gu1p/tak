use super::*;

use crate::container_engine::{ContainerEngine, engine_name};
use sha2::Digest;

pub(crate) fn deterministic_dockerfile_image_tag(
    engine: ContainerEngine,
    workspace_root: &Path,
    dockerfile: &PathRef,
    build_context: &PathRef,
) -> Result<String> {
    let cache_key =
        deterministic_dockerfile_cache_key(engine, workspace_root, dockerfile, build_context)?;
    Ok(format!("tak-runtime-{}", sha256_hex(cache_key.as_bytes())))
}

fn deterministic_dockerfile_cache_key(
    engine: ContainerEngine,
    workspace_root: &Path,
    dockerfile: &PathRef,
    build_context: &PathRef,
) -> Result<String> {
    let build_context_root = resolve_workspace_path_ref(workspace_root, build_context)
        .context("infra error: container lifecycle build failed: invalid build context")?;
    let dockerfile_path = resolve_workspace_path_ref(workspace_root, dockerfile)
        .context("infra error: container lifecycle build failed: invalid dockerfile path")?;
    let dockerfile_relative = dockerfile_path.strip_prefix(&build_context_root).context(
        "infra error: container lifecycle build failed: dockerfile must be within build context",
    )?;

    let mut files = Vec::new();
    collect_build_context_files(&build_context_root, &build_context_root, &mut files)?;
    files.sort_by(|left, right| left.0.cmp(&right.0));

    let mut hasher = sha2::Sha256::new();
    hasher.update(b"engine\0");
    hasher.update(engine_name(engine).as_bytes());
    hasher.update(b"\0dockerfile_path\0");
    hasher.update(normalize_archive_path(dockerfile_relative).as_bytes());
    hasher.update(b"\0dockerfile_bytes\0");
    hasher.update(fs::read(&dockerfile_path).with_context(|| {
        format!(
            "failed to read dockerfile for deterministic cache key: {}",
            dockerfile_path.display()
        )
    })?);

    hasher.update(b"\0context\0");
    for (relative, absolute, mode) in files {
        hasher.update(relative.as_bytes());
        hasher.update(b"\0");
        hasher.update(mode.to_le_bytes());
        hasher.update(b"\0");
        let bytes = fs::read(&absolute).with_context(|| {
            format!(
                "failed to read build context file for deterministic cache key: {}",
                absolute.display()
            )
        })?;
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(bytes);
        hasher.update(b"\0");
    }

    Ok(sha256_hex(&hasher.finalize()))
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(sha2::Sha256::digest(bytes))
}
