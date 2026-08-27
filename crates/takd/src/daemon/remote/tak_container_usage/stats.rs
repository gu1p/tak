use super::*;

pub(super) async fn sample_container_usage(
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
        return Err(anyhow!("Docker stats stream ended before a sample"));
    };
    usage_from_stats(&stats?)
}

fn usage_from_stats(stats: &Stats) -> Result<TakContainerUsageSnapshot> {
    Ok(TakContainerUsageSnapshot {
        cpu_cores: cpu_cores_from_stats(stats),
        memory_bytes: required_memory_usage(stats.memory_stats.usage)?,
        sampled_at: None,
        task_usage: HashMap::new(),
        attribution_complete: false,
    })
}

pub(super) fn required_memory_usage(memory_bytes: Option<u64>) -> Result<u64> {
    memory_bytes.ok_or_else(|| anyhow!("Docker stats sample omitted memory usage"))
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

pub(super) fn cpu_cores_from_deltas(
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
