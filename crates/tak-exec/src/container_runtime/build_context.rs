use super::*;

use bollard::errors::Error as BollardError;
use bollard::image::BuildImageOptions;
use tak_core::model::{ContainerRuntimeSourceSpec, PathAnchor, PathRef};

#[path = "build_context/archive.rs"]
mod archive;
#[path = "build_context/cache.rs"]
mod cache;
#[path = "build_context/key.rs"]
mod key;
#[path = "build_context/output.rs"]
mod output;

use archive::{build_context_archive, normalize_archive_path};
pub(crate) use key::deterministic_dockerfile_image_tag;
use output::{docker_build_error_message, emit_build_line, emit_docker_build_info};

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
