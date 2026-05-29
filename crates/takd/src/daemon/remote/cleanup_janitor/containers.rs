use std::collections::{BTreeSet, HashMap};

use anyhow::{Context, Result};
use bollard::container::{ListContainersOptions, RemoveContainerOptions};
use bollard::errors::Error as BollardError;
use bollard::{API_DEFAULT_VERSION, Docker};

use super::*;

pub(super) async fn cleanup_inactive_takd_containers(
    context: &RemoteNodeContext,
    active_jobs: &BTreeSet<String>,
) -> Result<()> {
    let docker = connect_cleanup_docker_client(&context.runtime_config())
        .await
        .context("connect container engine for remote container cleanup")?;
    let containers = list_takd_containers(&docker).await?;
    for container in containers {
        let labels = container.labels.unwrap_or_default();
        if labels.get("tak.owner").map(String::as_str) != Some("takd") {
            continue;
        }
        if container_belongs_to_active_job(&labels, active_jobs) {
            continue;
        }
        let Some(container_id) = container.id else {
            continue;
        };
        // `active_jobs` was snapshotted at the start of the sweep; listing the
        // containers can race a job that registered just afterward, whose
        // freshly created container would then look orphaned. Re-read the active
        // set immediately before removing so an in-flight job is never reaped.
        if container_belongs_to_active_job(&labels, &cleanup_protected_job_keys(context)?) {
            continue;
        }
        remove_takd_container(&docker, &container_id).await?;
    }
    Ok(())
}

async fn connect_cleanup_docker_client(runtime_config: &RemoteRuntimeConfig) -> Result<Docker> {
    let docker = if let Some(host) = runtime_config.docker_host() {
        if host.starts_with("unix://") || host.starts_with('/') {
            Docker::connect_with_unix(host, 120, API_DEFAULT_VERSION)?
        } else if host.starts_with("tcp://") || host.starts_with("http://") {
            Docker::connect_with_http(host, 120, API_DEFAULT_VERSION)?
        } else {
            Docker::connect_with_local_defaults()?
        }
    } else {
        Docker::connect_with_local_defaults()?
    };
    docker.ping().await?;
    Ok(docker)
}

async fn list_takd_containers(docker: &Docker) -> Result<Vec<bollard::models::ContainerSummary>> {
    let mut filters = HashMap::new();
    filters.insert("label".to_string(), vec!["tak.owner=takd".to_string()]);
    docker
        .list_containers(Some(ListContainersOptions {
            all: true,
            filters,
            ..Default::default()
        }))
        .await
        .context("list takd-owned containers")
}

fn container_belongs_to_active_job(
    labels: &HashMap<String, String>,
    active_jobs: &BTreeSet<String>,
) -> bool {
    labels
        .get("tak.submit_key")
        .map(|submit_key| active_jobs.contains(&sanitize_submit_idempotency_key(submit_key)))
        .unwrap_or(false)
}

async fn remove_takd_container(docker: &Docker, container_id: &str) -> Result<()> {
    match docker
        .remove_container(
            container_id,
            Some(RemoveContainerOptions {
                force: true,
                ..Default::default()
            }),
        )
        .await
    {
        Ok(()) => Ok(()),
        Err(err) if container_was_already_removed(&err) => Ok(()),
        Err(err) => {
            Err(err).with_context(|| format!("remove inactive takd container {container_id}"))
        }
    }
}

fn container_was_already_removed(error: &BollardError) -> bool {
    matches!(
        error,
        BollardError::DockerResponseServerError {
            status_code: 404,
            ..
        }
    )
}
