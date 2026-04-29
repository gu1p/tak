use std::path::Path;
use std::time::{Duration, Instant};

use takd::agent::{TransportHealth, TransportState, write_transport_health};

pub(crate) fn log_probe_progress(
    base_url: &str,
    stage: &str,
    attempt: u32,
    started_at: Instant,
    timeout: Duration,
    detail: impl AsRef<str>,
) {
    tracing::info!(
        "{}",
        startup_detail(base_url, stage, attempt, started_at, timeout, detail)
    );
}

pub(crate) fn record_probe_failure(
    state_root: Option<&Path>,
    base_url: &str,
    stage: &str,
    attempt: u32,
    started_at: Instant,
    timeout: Duration,
    detail: impl AsRef<str>,
) {
    let detail = startup_detail(base_url, stage, attempt, started_at, timeout, detail);
    tracing::warn!("{detail}");
    let Some(state_root) = state_root else {
        return;
    };
    if let Err(err) = write_transport_health(
        state_root,
        &TransportHealth::new(
            TransportState::Pending,
            Some(base_url.to_string()),
            Some(detail),
        ),
    ) {
        tracing::warn!("failed to persist tor transport startup detail: {err:#}");
    }
}

fn startup_detail(
    base_url: &str,
    stage: &str,
    attempt: u32,
    started_at: Instant,
    timeout: Duration,
    detail: impl AsRef<str>,
) -> String {
    format!(
        "Tor startup {stage} attempt {attempt} after {}ms of {}ms for {base_url}: {}",
        started_at.elapsed().as_millis(),
        timeout.as_millis(),
        detail.as_ref()
    )
}
