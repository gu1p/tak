use std::collections::HashMap;

use anyhow::{Context, Result};
use bollard::container::ListContainersOptions;
use bollard::{API_DEFAULT_VERSION, Docker};

use super::super::{RemoteRuntimeConfig, container_ownership};

pub(in crate::daemon::remote) async fn connect_docker_client(
    runtime_config: &RemoteRuntimeConfig,
) -> Result<Docker> {
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

pub(super) async fn list_active_takd_containers(
    docker: &Docker,
    node_id: &str,
) -> Result<Vec<bollard::models::ContainerSummary>> {
    let mut filters = HashMap::new();
    container_ownership::add_node_ownership_filter(&mut filters, node_id);
    filters.insert(
        "status".to_string(),
        vec!["running".to_string(), "paused".to_string()],
    );
    let containers = docker
        .list_containers(Some(ListContainersOptions {
            all: true,
            filters,
            ..Default::default()
        }))
        .await
        .context("list active takd-owned containers")?;
    Ok(containers
        .into_iter()
        .filter(|container| {
            container_ownership::labels_belong_to_node(container.labels.as_ref(), node_id)
        })
        .collect())
}
