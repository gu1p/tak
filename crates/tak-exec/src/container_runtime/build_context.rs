use super::*;

use bollard::errors::Error as BollardError;
use bollard::image::BuildImageOptions;
use bollard::models::BuildInfo;
use std::fs;
use tak_core::model::{ContainerRuntimeSourceSpec, PathAnchor, PathRef};

#[path = "build_context/cache.rs"]
mod cache;
#[path = "build_context/key.rs"]
mod key;

pub(crate) use key::deterministic_dockerfile_image_tag;

pub(super) async fn ensure_container_runtime_source(
    docker: &Docker,
    workspace_root: &Path,
    plan: &ContainerExecutionPlan,
    run_context: &ContainerStepRunContext<'_>,
) -> Result<()> {
    if plan.image_cache.is_some() {
        return cache::ensure_cached_container_runtime_source(
            docker,
            workspace_root,
            plan,
            run_context,
        )
        .await;
    }
    match &plan.source {
        ContainerRuntimeSourceSpec::Image { image } => ensure_container_image(docker, image).await,
        ContainerRuntimeSourceSpec::Dockerfile {
            dockerfile,
            build_context,
        } => {
            if docker.inspect_image(&plan.image).await.is_ok() {
                return Ok(());
            }
            build_container_image_from_dockerfile(
                docker,
                workspace_root,
                &plan.image,
                dockerfile,
                build_context,
                run_context,
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
    run_context: &ContainerStepRunContext<'_>,
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

    let dockerfile_relative = dockerfile_path.strip_prefix(&build_context_root).context(
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
        let build_info = match item {
            Ok(build_info) => build_info,
            Err(BollardError::DockerStreamError { error }) if !error.trim().is_empty() => {
                let error = error.trim().to_string();
                emit_build_line(&error, OutputStream::Stderr, run_context)?;
                bail!("infra error: container lifecycle build failed: {error}");
            }
            Err(error) => {
                return Err(error).context("infra error: container lifecycle build failed");
            }
        };
        emit_docker_build_info(&build_info, run_context)?;
        if let Some(error) = docker_build_error_message(&build_info) {
            bail!("infra error: container lifecycle build failed: {error}");
        }
    }
    Ok(())
}

fn emit_docker_build_info(
    build_info: &BuildInfo,
    run_context: &ContainerStepRunContext<'_>,
) -> Result<()> {
    if let Some(stream) = build_info
        .stream
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        crate::emit_task_output(
            run_context.output_observer,
            run_context.task_label,
            run_context.attempt,
            OutputStream::Stdout,
            stream.as_bytes(),
        )?;
    }
    if let Some(status) = docker_build_status_message(build_info) {
        emit_build_line(&status, OutputStream::Stdout, run_context)?;
    }
    if let Some(error) = docker_build_error_message(build_info) {
        emit_build_line(&error, OutputStream::Stderr, run_context)?;
    }
    Ok(())
}

fn emit_build_line(
    message: &str,
    stream: OutputStream,
    run_context: &ContainerStepRunContext<'_>,
) -> Result<()> {
    if message.is_empty() {
        return Ok(());
    }
    let mut line = message.as_bytes().to_vec();
    if !line.ends_with(b"\n") {
        line.push(b'\n');
    }
    crate::emit_task_output(
        run_context.output_observer,
        run_context.task_label,
        run_context.attempt,
        stream,
        &line,
    )
}

fn docker_build_status_message(build_info: &BuildInfo) -> Option<String> {
    let status = build_info.status.as_deref()?.trim();
    if status.is_empty() {
        return None;
    }
    let mut message = String::new();
    if let Some(id) = build_info
        .id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
    {
        message.push_str(id);
        message.push_str(": ");
    }
    message.push_str(status);
    if let Some(progress) = build_info
        .progress
        .as_deref()
        .map(str::trim)
        .filter(|progress| !progress.is_empty())
    {
        message.push(' ');
        message.push_str(progress);
    }
    Some(message)
}

fn docker_build_error_message(build_info: &BuildInfo) -> Option<String> {
    build_info
        .error_detail
        .as_ref()
        .and_then(|detail| detail.message.as_deref())
        .or(build_info.error.as_deref())
        .map(str::trim)
        .filter(|message| !message.is_empty())
        .map(str::to_string)
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
