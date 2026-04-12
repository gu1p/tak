use std::fs;
use bollard::image::BuildImageOptions;
use tak_core::model::{ContainerRuntimeSourceSpec, PathAnchor, PathRef};

async fn ensure_container_runtime_source(
    docker: &Docker,
    workspace_root: &Path,
    plan: &ContainerExecutionPlan,
) -> Result<()> {
    match &plan.source {
        ContainerRuntimeSourceSpec::Image { image } => ensure_container_image(docker, image).await,
        ContainerRuntimeSourceSpec::Dockerfile {
            dockerfile,
            build_context,
        } => {
            build_container_image_from_dockerfile(
                docker,
                workspace_root,
                &plan.image,
                dockerfile,
                build_context,
            )
            .await
        }
    }
}

async fn build_container_image_from_dockerfile(
    docker: &Docker,
    workspace_root: &Path,
    image: &str,
    dockerfile: &PathRef,
    build_context: &PathRef,
) -> Result<()> {
    let build_context_root = resolve_workspace_path_ref(workspace_root, build_context)
        .context("infra error: container lifecycle build failed: invalid build context")?;
    if !build_context_root.is_dir() {
        bail!(
            "infra error: container lifecycle build failed: build context {} is not a directory",
            build_context_root.display()
        );
    }

    let dockerfile_path = resolve_workspace_path_ref(workspace_root, dockerfile)
        .context("infra error: container lifecycle build failed: invalid dockerfile path")?;
    if !dockerfile_path.is_file() {
        bail!(
            "infra error: container lifecycle build failed: dockerfile {} does not exist",
            dockerfile_path.display()
        );
    }

    let dockerfile_relative = dockerfile_path
        .strip_prefix(&build_context_root)
        .context(
            "infra error: container lifecycle build failed: dockerfile must be within build context",
        )?;
    let archive = build_context_archive(&build_context_root)?;

    let mut stream = docker.build_image(
        BuildImageOptions {
            dockerfile: normalize_archive_path(dockerfile_relative),
            t: image.to_string(),
            rm: true,
            ..Default::default()
        },
        None,
        Some(archive.into()),
    );
    while let Some(item) = stream.next().await {
        item.context("infra error: container lifecycle build failed")?;
    }
    Ok(())
}

fn resolve_workspace_path_ref(workspace_root: &Path, path: &PathRef) -> Result<PathBuf> {
    if path.anchor != PathAnchor::Workspace {
        bail!(
            "unsupported non-workspace runtime path anchor during container execution: {:?}",
            path.anchor
        );
    }
    if path.path == "." {
        return Ok(workspace_root.to_path_buf());
    }
    Ok(workspace_root.join(&path.path))
}

fn build_context_archive(build_context_root: &Path) -> Result<Vec<u8>> {
    let mut files = Vec::new();
    collect_build_context_files(build_context_root, build_context_root, &mut files)?;
    files.sort_by(|left, right| left.0.cmp(&right.0));

    let mut archive = Vec::new();
    for (relative, absolute, mode) in files {
        let bytes = fs::read(&absolute).with_context(|| {
            format!("failed to read build context file {}", absolute.display())
        })?;
        append_tar_entry(&mut archive, &relative, &bytes, mode)?;
    }
    archive.extend([0_u8; 1024]);
    Ok(archive)
}

fn collect_build_context_files(
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
            format!("failed to read build context metadata for {}", path.display())
        })?;
        files.push((normalize_archive_path(relative), path, archive_mode(&metadata)));
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

fn normalize_archive_path(path: &Path) -> String {
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
