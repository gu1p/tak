use tak_proto::{CpuUsage, MemoryUsage, StorageUsage, SubmittedNeed};

use super::super::unix_epoch_ms;

pub(in crate::cli::remote_status) fn format_cpu(cpu: Option<&CpuUsage>) -> String {
    let Some(cpu) = cpu else {
        return "n/a".to_string();
    };
    let base = match cpu.utilization_percent {
        Some(percent) => format!("{percent:.1}%/{}c", cpu.logical_cores),
        None => format!("n/a/{}c", cpu.logical_cores),
    };
    match (cpu.tak_admission_available_cores, cpu.tak_reserved_cores) {
        (Some(available), Some(reserved)) => {
            format!("{base} tak={reserved:.2}c avail={available:.2}c")
        }
        (Some(available), None) => format!("{base} avail={available:.2}c"),
        _ => base,
    }
}

pub(in crate::cli::remote_status) fn format_memory(memory: Option<&MemoryUsage>) -> String {
    let Some(memory) = memory else {
        return "n/a".to_string();
    };
    let base = format!(
        "{}/{}",
        human_bytes(memory.used_bytes),
        human_bytes(memory.total_bytes)
    );
    let free = memory
        .available_bytes
        .map(|value| format!(" free={}", human_bytes(value)))
        .unwrap_or_default();
    let tak = memory
        .tak_reserved_bytes
        .map(|value| format!(" tak={}", human_bytes(value)))
        .unwrap_or_default();
    let available = memory
        .tak_admission_available_bytes
        .map(|value| format!(" avail={}", human_bytes(value)))
        .unwrap_or_default();
    format!("{base}{free}{tak}{available}")
}

pub(in crate::cli::remote_status) fn format_storage(storage: Option<&StorageUsage>) -> String {
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

pub(in crate::cli::remote_status) fn format_image_cache(
    cache: Option<&tak_proto::ImageCacheStatus>,
) -> String {
    let Some(cache) = cache else {
        return "n/a".to_string();
    };
    format!(
        "{:.1}GB/{:.1}GB",
        cache.used_bytes as f64 / 1_000_000_000.0,
        cache.budget_bytes as f64 / 1_000_000_000.0,
    )
}

pub(in crate::cli::remote_status) fn format_needs(needs: &[SubmittedNeed]) -> String {
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

pub(in crate::cli::remote_status) fn human_bytes(bytes: u64) -> String {
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

pub(in crate::cli::remote_status) fn age_since(started_at_ms: i64) -> String {
    let delta_s = unix_epoch_ms().saturating_sub(started_at_ms).max(0) / 1000;
    if delta_s >= 3600 {
        return format!("{}h{}m", delta_s / 3600, (delta_s % 3600) / 60);
    }
    if delta_s >= 60 {
        return format!("{}m{}s", delta_s / 60, delta_s % 60);
    }
    format!("{delta_s}s")
}
