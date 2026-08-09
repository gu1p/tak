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
            let paused = summary.state.as_deref() == Some("paused");
            Some(ManagedContainer {
                id,
                created,
                has_timeout,
                paused,
            })
        })
        .collect()
}

pub(super) async fn list_managed_takd_containers(docker: &Docker) -> Result<Vec<ContainerSummary>> {
    let mut filters = HashMap::new();
    filters.insert("label".to_string(), vec!["tak.owner=takd".to_string()]);
    // Both states matter: running containers are pause candidates; paused ones
    // are unpause candidates. (A paused container's status is `paused`, not
    // `running`, so a running-only filter would lose track of what we froze.)
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
        Err(err) => tracing::warn!(container_id = %id, "memory pressure: pause failed: {err}"),
    }
}

pub(super) async fn unpause_container(docker: &Docker, id: &str) {
    match docker.unpause_container(id).await {
        Ok(()) => tracing::info!(container_id = %id, "memory pressure: unpaused container"),
        // A 404 / not-paused means the container already finished or was resumed
        // elsewhere — harmless; the next tick reconciles from engine state.
        Err(err) => tracing::debug!(container_id = %id, "memory pressure: unpause: {err}"),
    }
}
