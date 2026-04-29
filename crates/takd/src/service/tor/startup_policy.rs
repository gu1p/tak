use std::time::Duration;

const DEFAULT_TOR_STARTUP_PROBE_TIMEOUT: Duration = Duration::from_secs(120);
const DEFAULT_TOR_STARTUP_PROBE_BACKOFF: Duration = Duration::from_secs(1);
const DEFAULT_TOR_STARTUP_PROBE_MAX_BACKOFF: Duration = Duration::from_secs(15);

pub(super) struct TorStartupProbeRetryPolicy {
    pub(super) timeout: Duration,
    pub(super) initial_backoff: Duration,
    pub(super) max_backoff: Duration,
}

pub(super) struct CappedExponentialBackoff {
    current_backoff: Duration,
    max_backoff: Duration,
}

pub(super) fn startup_probe_retry_policy() -> TorStartupProbeRetryPolicy {
    let initial_backoff = env_duration_ms(
        "TAKD_TOR_STARTUP_PROBE_BACKOFF_MS",
        DEFAULT_TOR_STARTUP_PROBE_BACKOFF,
    );
    let max_backoff = env_duration_ms(
        "TAKD_TOR_STARTUP_PROBE_MAX_BACKOFF_MS",
        DEFAULT_TOR_STARTUP_PROBE_MAX_BACKOFF,
    )
    .max(initial_backoff);
    TorStartupProbeRetryPolicy {
        timeout: env_duration_ms(
            "TAKD_TOR_STARTUP_PROBE_TIMEOUT_MS",
            DEFAULT_TOR_STARTUP_PROBE_TIMEOUT,
        ),
        initial_backoff,
        max_backoff,
    }
}

impl CappedExponentialBackoff {
    pub(super) fn new(initial_backoff: Duration, max_backoff: Duration) -> Self {
        Self {
            current_backoff: initial_backoff,
            max_backoff: max_backoff.max(initial_backoff),
        }
    }

    pub(super) fn next_backoff(&mut self) -> Duration {
        let delay = self.current_backoff;
        self.current_backoff = self.current_backoff.saturating_mul(2).min(self.max_backoff);
        delay
    }
}

fn env_duration_ms(name: &str, default: Duration) -> Duration {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or(default)
}

#[path = "startup_policy_tests.rs"]
mod startup_policy_tests;
