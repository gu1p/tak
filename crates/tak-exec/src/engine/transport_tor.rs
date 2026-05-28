use std::time::Duration;

const DEFAULT_TOR_CONNECT_TIMEOUT: Duration = Duration::from_secs(120);

pub(super) fn tor_connect_timeout() -> Duration {
    env_duration_ms(
        "TAK_TEST_TOR_PROBE_TIMEOUT_MS",
        "TAK_TOR_PROBE_TIMEOUT_MS",
        DEFAULT_TOR_CONNECT_TIMEOUT,
    )
}

fn env_duration_ms(test_name: &str, live_name: &str, default: Duration) -> Duration {
    std::env::var(test_name)
        .ok()
        .or_else(|| std::env::var(live_name).ok())
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or(default)
}
