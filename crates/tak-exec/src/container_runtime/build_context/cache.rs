use super::*;

use bollard::errors::Error as BollardError;

use crate::container_engine::engine_name;
use crate::container_runtime::foundation::pull_container_image;
use crate::image_cache::{
    ImageCacheRecord, image_cache_entry, mutable_tag_entry_is_expired, record_image_cache_entry,
};

pub(super) async fn ensure_cached_container_runtime_source(
    docker: &Docker,
    workspace_root: &Path,
    plan: &ContainerExecutionPlan,
) -> Result<()> {
    match &plan.source {
        ContainerRuntimeSourceSpec::Image { image } => {
            if let Some(refreshed) = ensure_cached_image_source(docker, plan, image).await? {
                record_cached_image(docker, plan, image, refreshed).await?;
            }
            Ok(())
        }
        ContainerRuntimeSourceSpec::Dockerfile {
            dockerfile,
            build_context,
        } => {
            let mut refreshed = false;
            if docker.inspect_image(&plan.image).await.is_err() {
                build_container_image_from_dockerfile(
                    docker,
                    workspace_root,
                    &plan.image,
                    dockerfile,
                    build_context,
                )
                .await?;
                refreshed = true;
            }
            record_cached_image(docker, plan, &plan.image, refreshed).await
        }
    }
}

async fn ensure_cached_image_source(
    docker: &Docker,
    plan: &ContainerExecutionPlan,
    image: &str,
) -> Result<Option<bool>> {
    let Some(cache) = plan.image_cache.as_ref() else {
        ensure_container_image(docker, image).await?;
        return Ok(None);
    };
    let tracked_entry =
        image_cache_entry(&cache.options, engine_name(plan.engine), &cache.cache_key)?;
    if let Some(entry) = tracked_entry.as_ref()
        && mutable_tag_entry_is_expired(&cache.options, entry)
    {
        pull_container_image(docker, image).await?;
        return Ok(Some(true));
    }

    match docker.inspect_image(image).await {
        Ok(_) => Ok(Some(false)),
        Err(BollardError::DockerResponseServerError {
            status_code: 404, ..
        }) => {
            pull_container_image(docker, image).await?;
            Ok(Some(true))
        }
        Err(err) => {
            Err(err).context("infra error: container lifecycle pull failed: inspect image failed")
        }
    }
}

async fn record_cached_image(
    docker: &Docker,
    plan: &ContainerExecutionPlan,
    image_ref: &str,
    refreshed: bool,
) -> Result<()> {
    let Some(cache) = plan.image_cache.as_ref() else {
        return Ok(());
    };
    let inspect = docker
        .inspect_image(image_ref)
        .await
        .context("infra error: container lifecycle cache failed: inspect cached image failed")?;
    let image_id = inspect.id.unwrap_or_else(|| image_ref.to_string());
    let size_bytes = inspect
        .size
        .and_then(|value| u64::try_from(value.max(0)).ok())
        .unwrap_or(0);
    record_image_cache_entry(
        &cache.options,
        ImageCacheRecord {
            engine: engine_name(plan.engine),
            cache_key: &cache.cache_key,
            source_kind: &cache.source_kind,
            image_ref,
            image_id: &image_id,
            size_bytes,
            refreshed,
        },
    )?;
    Ok(())
}
