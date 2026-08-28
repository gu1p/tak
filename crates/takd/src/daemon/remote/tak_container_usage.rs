use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyhow::{Context, Result, anyhow};
use bollard::Docker;
use bollard::container::{Stats, StatsOptions};
use futures::StreamExt;
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

use super::RemoteNodeContext;
use super::resource_admission::{ResourceCapacity, SharedResourceAdmission};
use super::runtime::RemoteRuntimeConfig;
use super::status_resources::{host_cpu_cores_used, non_tak_cpu_cores, non_tak_memory_bytes};

mod docker;
mod stats;
#[path = "tak_container_usage_tests.rs"]
mod tests;

pub(super) use docker::connect_docker_client;
use docker::list_active_takd_containers;
use stats::sample_container_usage;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct TakTaskUsageSnapshot {
    pub(crate) cpu_cores: f64,
    pub(crate) memory_bytes: u64,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct TakContainerUsageSnapshot {
    pub(crate) cpu_cores: f64,
    pub(crate) memory_bytes: u64,
    pub(crate) sampled_at: Option<Instant>,
    pub(crate) task_usage: HashMap<String, TakTaskUsageSnapshot>,
    pub(crate) attribution_complete: bool,
}

#[derive(Clone, Default)]
pub(crate) struct SharedTakContainerUsage {
    inner: Arc<Mutex<TakContainerUsageSnapshot>>,
}

impl SharedTakContainerUsage {
    pub(crate) fn latest(&self) -> TakContainerUsageSnapshot {
        self.inner
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    fn update(&self, mut snapshot: TakContainerUsageSnapshot) {
        snapshot.sampled_at = Some(Instant::now());
        if let Ok(mut guard) = self.inner.lock() {
            *guard = snapshot;
        }
    }
}

pub(crate) fn spawn_tak_container_usage_sampler(
    context: RemoteNodeContext,
) -> tokio::task::JoinHandle<()> {
    let runtime_config = context.runtime_config();
    let usage = context.tak_container_usage();
    let admission = context.resource_admission();
    let node_id = context.node_info().map(|node| node.node_id);
    let sample_containers = !tak_core::mock::mock_container_enabled();
    let ignore_host_usage = runtime_config.ignore_host_usage_for_tests() || !sample_containers;
    tokio::spawn(async move {
        let node_id = match node_id {
            Ok(node_id) => node_id,
            Err(error) => {
                tracing::error!(
                    "Tak container resource sampler cannot read node identity: {error}"
                );
                return;
            }
        };
        let mut host = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );
        host.refresh_memory();
        host.refresh_cpu_all();
        let mut ticker = tokio::time::interval(runtime_config.resource_sample_interval());
        loop {
            ticker.tick().await;
            if sample_containers {
                match sample_tak_container_usage(&runtime_config, &node_id).await {
                    Ok(sample) => {
                        usage.update(sample);
                    }
                    Err(error) => {
                        tracing::warn!(
                            "Tak container resource sample failed; retaining prior sample: {error:#}"
                        )
                    }
                }
            }
            if let Err(error) = update_host_usage(ignore_host_usage, &mut host, &usage, &admission)
            {
                tracing::warn!(
                    "host resource sample failed; retaining prior admission sample: {error:#}"
                );
            }
        }
    })
}

fn update_host_usage(
    ignore_host_usage: bool,
    host: &mut System,
    usage: &SharedTakContainerUsage,
    admission: &SharedResourceAdmission,
) -> Result<()> {
    if ignore_host_usage {
        return admission.update_host_usage(
            ResourceCapacity {
                cpu_cores: 0.0,
                memory_mb: 0,
            },
            u64::MAX,
        );
    }
    host.refresh_memory();
    host.refresh_cpu_usage();
    let tak = usage.latest();
    let logical_cores = u32::try_from(host.cpus().len()).unwrap_or(u32::MAX);
    let host_cpu = host_cpu_cores_used(f64::from(host.global_cpu_usage()), logical_cores);
    let host_used_memory = host.total_memory().saturating_sub(host.available_memory());
    admission.update_host_usage(
        ResourceCapacity {
            cpu_cores: non_tak_cpu_cores(host_cpu, tak.cpu_cores),
            memory_mb: non_tak_memory_bytes(host_used_memory, tak.memory_bytes)
                .div_ceil(1024 * 1024),
        },
        host.available_memory().div_ceil(1024 * 1024),
    )
}

async fn sample_tak_container_usage(
    runtime_config: &RemoteRuntimeConfig,
    node_id: &str,
) -> Result<TakContainerUsageSnapshot> {
    let docker = connect_docker_client(runtime_config).await?;
    let containers = list_active_takd_containers(&docker, node_id).await?;
    let mut total = TakContainerUsageSnapshot {
        attribution_complete: true,
        ..TakContainerUsageSnapshot::default()
    };
    for container in containers {
        let submit_key = container
            .labels
            .as_ref()
            .and_then(|labels| labels.get("tak.submit_key"))
            .cloned();
        let Some(container_id) = container.id else {
            total.attribution_complete = false;
            continue;
        };
        let usage = sample_container_usage(&docker, &container_id)
            .await
            .with_context(|| format!("sample Tak container {container_id}"))?;
        total.cpu_cores += usage.cpu_cores;
        total.memory_bytes = total.memory_bytes.saturating_add(usage.memory_bytes);
        if let Some(submit_key) = submit_key {
            let task = total.task_usage.entry(submit_key).or_default();
            task.cpu_cores += usage.cpu_cores;
            task.memory_bytes = task.memory_bytes.saturating_add(usage.memory_bytes);
        } else {
            total.attribution_complete = false;
        }
    }
    Ok(total)
}
