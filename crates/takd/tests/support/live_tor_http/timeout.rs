use std::time::Duration;

const DEFAULT_LIVE_TOR_TEST_TIMEOUT_SECS: u64 = 420;

pub fn live_tor_test_timeout() -> Duration {
    std::env::var("TAK_LIVE_TOR_TEST_TIMEOUT_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or_else(|| Duration::from_secs(DEFAULT_LIVE_TOR_TEST_TIMEOUT_SECS))
}

pub fn format_live_tor_wait_timeout(base_url: &str, last_error: Option<&anyhow::Error>) -> String {
    let last_attempt = last_error
        .map(|err| format!("; last attempt: {err:#}"))
        .unwrap_or_default();
    format!("timed out waiting for separate Arti client to reach {base_url}{last_attempt}")
}
