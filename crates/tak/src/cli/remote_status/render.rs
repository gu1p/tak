use tak_proto::{CpuUsage, MemoryUsage, StorageUsage, SubmittedNeed};

use super::{RemoteStatusResult, unix_epoch_ms};

pub(super) fn render_snapshot(results: &[RemoteStatusResult]) -> String {
    let mut output = String::from("Nodes\n");
    for result in results {
        let transport = result
            .status
            .as_ref()
            .and_then(|status| status.node.as_ref().map(|node| node.transport.as_str()))
            .unwrap_or(result.remote.transport.as_str());
        if let Some(status) = &result.status {
            let node = status.node.as_ref();
            let state = node
                .map(|node| node.transport_state.as_str())
                .filter(|value| !value.is_empty())
                .unwrap_or("ready");
            let detail = node
                .map(|node| node.transport_detail.as_str())
                .filter(|value| !value.is_empty())
                .map(|value| format!(" detail={value}"))
                .unwrap_or_default();
            output.push_str(&format!(
                "{} transport={} state={} jobs={} cpu={} ram={} storage={} tak_exec={} status=ok{}\n",
                result.remote.node_id,
                transport,
                state,
                status.active_jobs.len(),
                format_cpu(status.cpu.as_ref()),
                format_memory(status.memory.as_ref()),
                format_storage(status.storage.as_ref()),
                status
                    .storage
                    .as_ref()
                    .map(|value| human_bytes(value.tak_execution_bytes))
                    .unwrap_or_else(|| "n/a".to_string()),
                detail,
            ));
            output.push_str(&format!(
                "  image_cache={} image_cache_entries={}\n",
                format_image_cache(status.image_cache.as_ref()),
                status
                    .image_cache
                    .as_ref()
                    .map(|value| value.entry_count.to_string())
                    .unwrap_or_else(|| "n/a".to_string()),
            ));
        } else {
            output.push_str(&format!(
                "{} transport={} jobs=n/a cpu=n/a ram=n/a storage=n/a tak_exec=n/a image_cache=n/a status={}\n",
                result.remote.node_id,
                transport,
                result.error.as_deref().unwrap_or("unknown_error"),
            ));
        }
    }

    output.push_str("\nActive Jobs\n");
    let mut any_jobs = false;
    for result in results {
        let Some(status) = &result.status else {
            continue;
        };
        for job in &status.active_jobs {
            any_jobs = true;
            output.push_str(&format!(
                "{} {} attempt={} age={} needs={} exec_root={} runtime={}\n",
                result.remote.node_id,
                job.task_label,
                job.attempt,
                age_since(job.started_at_ms),
                format_needs(&job.needs),
                human_bytes(job.execution_root_bytes),
                job.runtime.as_deref().unwrap_or("none"),
            ));
        }
    }
    if !any_jobs {
        output.push_str("(none)\n");
    }
    output
}

fn format_cpu(cpu: Option<&CpuUsage>) -> String {
    let Some(cpu) = cpu else {
        return "n/a".to_string();
    };
    match cpu.utilization_percent {
        Some(percent) => format!("{percent:.1}%/{}c", cpu.logical_cores),
        None => format!("n/a/{}c", cpu.logical_cores),
    }
}

fn format_memory(memory: Option<&MemoryUsage>) -> String {
    let Some(memory) = memory else {
        return "n/a".to_string();
    };
    format!(
        "{}/{}",
        human_bytes(memory.used_bytes),
        human_bytes(memory.total_bytes)
    )
}

fn format_storage(storage: Option<&StorageUsage>) -> String {
    let Some(storage) = storage else {
        return "n/a".to_string();
    };
    format!(
        "{}/{} free={}",
        human_bytes(storage.used_bytes),
        human_bytes(storage.total_bytes),
        human_bytes(storage.available_bytes),
    )
}

fn format_image_cache(cache: Option<&tak_proto::ImageCacheStatus>) -> String {
    let Some(cache) = cache else {
        return "n/a".to_string();
    };
    format!(
        "{}/{}",
        human_decimal_gb(cache.used_bytes),
        human_decimal_gb(cache.budget_bytes)
    )
}

fn format_needs(needs: &[SubmittedNeed]) -> String {
    if needs.is_empty() {
        return "(none)".to_string();
    }
    needs.iter().map(format_need).collect::<Vec<_>>().join(",")
}

fn format_need(need: &SubmittedNeed) -> String {
    let scope_key = need
        .scope_key
        .as_deref()
        .map(|value| format!("/{value}"))
        .unwrap_or_default();
    format!(
        "{}({}{})={}",
        need.name,
        need.scope,
        scope_key,
        format_slots(need.slots)
    )
}

fn format_slots(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        format!("{value:.2}")
    }
}

fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit_index = 0_usize;
    while value >= 1024.0 && unit_index + 1 < UNITS.len() {
        value /= 1024.0;
        unit_index += 1;
    }
    if unit_index == 0 {
        format!("{bytes}{}", UNITS[unit_index])
    } else {
        format!("{value:.1}{}", UNITS[unit_index])
    }
}

fn human_decimal_gb(bytes: u64) -> String {
    format!("{:.1}GB", bytes as f64 / 1_000_000_000.0)
}

fn age_since(started_at_ms: i64) -> String {
    let delta_s = unix_epoch_ms().saturating_sub(started_at_ms).max(0) / 1000;
    if delta_s >= 3600 {
        return format!("{}h{}m", delta_s / 3600, (delta_s % 3600) / 60);
    }
    if delta_s >= 60 {
        return format!("{}m{}s", delta_s / 60, delta_s % 60);
    }
    format!("{delta_s}s")
}
