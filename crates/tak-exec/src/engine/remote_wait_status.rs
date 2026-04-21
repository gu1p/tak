use super::*;

use prost::Message;
use tak_proto::{CpuUsage, MemoryUsage, NodeStatusResponse};

pub(crate) async fn render_remote_wait_heartbeat(
    target: &StrictRemoteTarget,
    elapsed_s: u64,
) -> String {
    let summary = fetch_remote_wait_telemetry_summary(target)
        .await
        .unwrap_or_else(|| "node telemetry unavailable".to_string());
    format!(
        "remote task still running on {}; no new output for {}s; {}",
        target.node_id, elapsed_s, summary
    )
}

async fn fetch_remote_wait_telemetry_summary(target: &StrictRemoteTarget) -> Option<String> {
    let fetch = async {
        let (status, response_body) = remote_protocol_http_request(
            target,
            "GET",
            "/v1/node/status",
            None,
            "node status",
            remote_wait_status_timeout(),
        )
        .await?;
        if status != 200 {
            bail!(
                "infra error: remote node {} node status failed with HTTP {}",
                target.node_id,
                status
            );
        }
        let parsed = NodeStatusResponse::decode(response_body.as_slice()).with_context(|| {
            format!(
                "infra error: remote node {} returned invalid protobuf for node status",
                target.node_id
            )
        })?;
        Ok::<_, anyhow::Error>(format!(
            "jobs={} cpu={} ram={}",
            parsed.active_jobs.len(),
            format_remote_wait_cpu(parsed.cpu.as_ref()),
            format_remote_wait_memory(parsed.memory.as_ref()),
        ))
    };

    tokio::time::timeout(remote_wait_status_timeout(), fetch)
        .await
        .ok()
        .and_then(|result| result.ok())
}

pub(crate) fn remote_wait_heartbeat_interval() -> Duration {
    const DEFAULT_REMOTE_WAIT_HEARTBEAT: Duration = Duration::from_secs(30);

    std::env::var("TAK_TEST_REMOTE_WAIT_HEARTBEAT_MS")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .map(Duration::from_millis)
        .unwrap_or(DEFAULT_REMOTE_WAIT_HEARTBEAT)
}

fn remote_wait_status_timeout() -> Duration {
    Duration::from_secs(2)
}

fn format_remote_wait_cpu(cpu: Option<&CpuUsage>) -> String {
    let Some(cpu) = cpu else {
        return "n/a".to_string();
    };
    match cpu.utilization_percent {
        Some(percent) => format!("{percent:.1}%/{}c", cpu.logical_cores),
        None => format!("n/a/{}c", cpu.logical_cores),
    }
}

fn format_remote_wait_memory(memory: Option<&MemoryUsage>) -> String {
    let Some(memory) = memory else {
        return "n/a".to_string();
    };
    format!(
        "{}/{}",
        human_remote_wait_bytes(memory.used_bytes),
        human_remote_wait_bytes(memory.total_bytes)
    )
}

fn human_remote_wait_bytes(bytes: u64) -> String {
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
