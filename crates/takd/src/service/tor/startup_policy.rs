use std::time::Duration;

const DEFAULT_TOR_STARTUP_PROBE_TIMEOUT: Duration = Duration::from_secs(120);
const DEFAULT_TOR_STARTUP_PROBE_BACKOFF: Duration = Duration::from_secs(1);

pub(super) struct TorStartupProbeRetryPolicy {
    pub(super) timeout: Duration,
    pub(super) backoff: Duration,
}

pub(super) fn startup_probe_retry_policy() -> TorStartupProbeRetryPolicy {
    TorStartupProbeRetryPolicy {
        timeout: env_duration_ms(
            "TAKD_TOR_STARTUP_PROBE_TIMEOUT_MS",
            DEFAULT_TOR_STARTUP_PROBE_TIMEOUT,
        ),
        backoff: env_duration_ms(
            "TAKD_TOR_STARTUP_PROBE_BACKOFF_MS",
            DEFAULT_TOR_STARTUP_PROBE_BACKOFF,
        ),
    }
}

fn env_duration_ms(name: &str, default: Duration) -> Duration {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or(default)
}
