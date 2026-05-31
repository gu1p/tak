use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result};
use bollard::container::{ListContainersOptions, Stats, StatsOptions};
use bollard::{API_DEFAULT_VERSION, Docker};
use futures::StreamExt;

use super::runtime::RemoteRuntimeConfig;

const SAMPLE_INTERVAL: Duration = Duration::from_secs(2);

#[path = "tak_container_usage_tests.rs"]
mod tests;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct TakContainerUsageSnapshot {
    pub(crate) cpu_cores: f64,
    pub(crate) memory_bytes: u64,
}

#[derive(Clone, Default)]
pub(crate) struct SharedTakContainerUsage {
    inner: Arc<Mutex<TakContainerUsageSnapshot>>,
}

impl SharedTakContainerUsage {
    pub(crate) fn latest(&self) -> TakContainerUsageSnapshot {
        self.inner.lock().map(|guard| *guard).unwrap_or_default()
    }

    fn update(&self, snapshot: TakContainerUsageSnapshot) {
        if let Ok(mut guard) = self.inner.lock() {
            *guard = snapshot;
        }
    }
}

pub(crate) fn spawn_tak_container_usage_sampler(
    runtime_config: RemoteRuntimeConfig,
    usage: SharedTakContainerUsage,
) {
    if tak_core::mock::mock_container_enabled() {
        return;
    }
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(SAMPLE_INTERVAL);
        loop {
            ticker.tick().await;
            let sample = sample_tak_container_usage(&runtime_config)
                .await
                .unwrap_or_default();
            usage.update(sample);
        }
    });
}

async fn sample_tak_container_usage(
    runtime_config: &RemoteRuntimeConfig,
) -> Result<TakContainerUsageSnapshot> {
    let docker = connect_docker_client(runtime_config).await?;
    let containers = list_running_takd_containers(&docker).await?;
    let mut total = TakContainerUsageSnapshot::default();
    for container in containers {
        let Some(container_id) = container.id else {
            continue;
        };
        let usage = sample_container_usage(&docker, &container_id)
            .await
            .unwrap_or_default();
        total.cpu_cores += usage.cpu_cores;
        total.memory_bytes = total.memory_bytes.saturating_add(usage.memory_bytes);
    }
    Ok(total)
}

async fn connect_docker_client(runtime_config: &RemoteRuntimeConfig) -> Result<Docker> {
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

async fn list_running_takd_containers(
    docker: &Docker,
) -> Result<Vec<bollard::models::ContainerSummary>> {
    let mut filters = HashMap::new();
    filters.insert("label".to_string(), vec!["tak.owner=takd".to_string()]);
    filters.insert("status".to_string(), vec!["running".to_string()]);
    docker
        .list_containers(Some(ListContainersOptions {
            all: true,
            filters,
            ..Default::default()
        }))
        .await
        .context("list running takd-owned containers")
}

async fn sample_container_usage(
    docker: &Docker,
    container_id: &str,
) -> Result<TakContainerUsageSnapshot> {
    let mut stream = docker
        .stats(
            container_id,
            Some(StatsOptions {
                stream: false,
                one_shot: false,
            }),
        )
        .take(1);
    let Some(stats) = stream.next().await else {
        return Ok(TakContainerUsageSnapshot::default());
    };
    Ok(usage_from_stats(&stats?))
}

fn usage_from_stats(stats: &Stats) -> TakContainerUsageSnapshot {
    TakContainerUsageSnapshot {
        cpu_cores: cpu_cores_from_stats(stats),
        memory_bytes: stats.memory_stats.usage.unwrap_or(0),
    }
}

fn cpu_cores_from_stats(stats: &Stats) -> f64 {
    let per_cpu_count = stats
        .cpu_stats
        .cpu_usage
        .percpu_usage
        .as_ref()
        .map(Vec::len);
    cpu_cores_from_deltas(
        stats.cpu_stats.cpu_usage.total_usage,
        stats.precpu_stats.cpu_usage.total_usage,
        stats.cpu_stats.system_cpu_usage,
        stats.precpu_stats.system_cpu_usage,
        stats.cpu_stats.online_cpus,
        per_cpu_count,
    )
}

fn cpu_cores_from_deltas(
    cpu_total: u64,
    pre_cpu_total: u64,
    system_total: Option<u64>,
    pre_system_total: Option<u64>,
    online_cpus: Option<u64>,
    per_cpu_count: Option<usize>,
) -> f64 {
    let cpu_delta = cpu_total.saturating_sub(pre_cpu_total);
    let system_delta = system_total
        .zip(pre_system_total)
        .map(|(current, previous)| current.saturating_sub(previous))
        .unwrap_or(0);
    if cpu_delta == 0 || system_delta == 0 {
        return 0.0;
    }
    let cpu_count = online_cpus
        .or_else(|| per_cpu_count.and_then(|count| u64::try_from(count).ok()))
        .unwrap_or(1);
    (cpu_delta as f64 / system_delta as f64) * cpu_count as f64
}
