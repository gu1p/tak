use std::collections::HashMap;

use anyhow::{Context, Result};
use bollard::Docker;
use bollard::container::ListContainersOptions;
use bollard::models::ContainerSummary;

use super::policy::ManagedContainer;

pub(super) fn managed_containers(summaries: &[ContainerSummary]) -> Vec<ManagedContainer> {
    summaries
        .iter()
        .filter_map(|summary| {
            let id = summary.id.clone()?;
            let created = summary.created.unwrap_or(0);
            let has_timeout = summary
                .labels
                .as_ref()
                .and_then(|labels| labels.get("tak.timeout_s"))
                .and_then(|value| value.parse::<u64>().ok())
                .is_some_and(|seconds| seconds > 0);
            Some(ManagedContainer {
                id,
                created,
                has_timeout,
                paused: summary.state.as_deref() == Some("paused"),
            })
        })
        .collect()
}

pub(super) async fn list_managed_takd_containers(docker: &Docker) -> Result<Vec<ContainerSummary>> {
    let mut filters = HashMap::new();
    filters.insert("label".to_string(), vec!["tak.owner=takd".to_string()]);
    filters.insert(
        "status".to_string(),
        vec!["running".to_string(), "paused".to_string()],
    );
    docker
        .list_containers(Some(ListContainersOptions {
            all: true,
            filters,
            ..Default::default()
        }))
        .await
        .context("list managed takd containers")
}

pub(super) async fn pause_container(docker: &Docker, id: &str) {
    match docker.pause_container(id).await {
        Ok(()) => tracing::info!(container_id = %id, "memory pressure: paused container"),
        Err(error) => {
            tracing::warn!(container_id = %id, "memory pressure: pause failed: {error}")
        }
    }
}

pub(super) async fn unpause_container(docker: &Docker, id: &str) -> bool {
    match docker.unpause_container(id).await {
        Ok(()) => {
            tracing::info!(container_id = %id, "memory pressure: unpaused container");
            true
        }
        Err(error) => {
            tracing::warn!(container_id = %id, "memory pressure: unpause failed: {error}");
            false
        }
    }
}
