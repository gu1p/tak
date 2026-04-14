use std::fs;
use std::path::Path;
use std::time::Duration;

const DEFAULT_PROBE_INTERVAL_MS: u64 = 15_000;
const DEFAULT_PROBE_TIMEOUT_MS: u64 = 10_000;
const DEFAULT_PROBE_BACKOFF_MS: u64 = 500;
const DEFAULT_FAILURE_THRESHOLD: u32 = 3;
const DEFAULT_INITIAL_BACKOFF_MS: u64 = 5_000;
const DEFAULT_MAX_BACKOFF_MS: u64 = 60_000;
const DEFAULT_STARTUP_SESSION_TIMEOUT_MS: u64 = 60_000;
const TEST_FORCE_RECOVERY_MARKER: &str = ".takd-test-force-recovery-consumed";
const TEST_STARTUP_FAILURE_MARKER: &str = ".takd-test-startup-failure-consumed";

#[derive(Debug, Clone)]
pub(crate) struct TorRecoveryConfig {
    pub(crate) probe_interval: Duration,
    pub(crate) probe_timeout: Duration,
    pub(crate) probe_backoff: Duration,
    pub(crate) failure_threshold: u32,
    pub(crate) initial_backoff: Duration,
    pub(crate) max_backoff: Duration,
}

pub(crate) fn tor_recovery_config() -> TorRecoveryConfig {
    let initial_backoff = env_duration_ms(
        "TAKD_TOR_RECOVERY_INITIAL_BACKOFF_MS",
        DEFAULT_INITIAL_BACKOFF_MS,
    );
    let max_backoff = env_duration_ms("TAKD_TOR_RECOVERY_MAX_BACKOFF_MS", DEFAULT_MAX_BACKOFF_MS)
        .max(initial_backoff);
    TorRecoveryConfig {
        probe_interval: env_duration_ms(
            "TAKD_TOR_RECOVERY_PROBE_INTERVAL_MS",
            DEFAULT_PROBE_INTERVAL_MS,
        ),
        probe_timeout: env_duration_ms(
            "TAKD_TOR_RECOVERY_PROBE_TIMEOUT_MS",
            DEFAULT_PROBE_TIMEOUT_MS,
        ),
        probe_backoff: env_duration_ms(
            "TAKD_TOR_RECOVERY_PROBE_BACKOFF_MS",
            DEFAULT_PROBE_BACKOFF_MS,
        ),
        failure_threshold: std::env::var("TAKD_TOR_RECOVERY_FAILURE_THRESHOLD")
            .ok()
            .and_then(|value| value.trim().parse::<u32>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(DEFAULT_FAILURE_THRESHOLD),
        initial_backoff,
        max_backoff,
    }
}

pub(crate) fn take_test_force_recovery_after(state_root: &Path) -> Option<Duration> {
    let delay_ms = std::env::var("TAKD_TEST_TOR_FORCE_RECOVERY_AFTER_MS")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)?;
    let marker = state_root.join(TEST_FORCE_RECOVERY_MARKER);
    if marker.exists() {
        return None;
    }
    fs::create_dir_all(state_root).ok()?;
    fs::write(marker, b"1").ok()?;
    Some(Duration::from_millis(delay_ms))
}

pub(crate) fn take_test_startup_failure(state_root: &Path) -> bool {
    let enabled = std::env::var("TAKD_TEST_TOR_FAIL_STARTUP_ONCE")
        .ok()
        .is_some_and(|value| {
            let value = value.trim();
            value == "1" || value.eq_ignore_ascii_case("true")
        });
    if !enabled {
        return false;
    }
    let marker = state_root.join(TEST_STARTUP_FAILURE_MARKER);
    if marker.exists() {
        return false;
    }
    fs::create_dir_all(state_root).ok();
    fs::write(marker, b"1").ok();
    true
}

pub(crate) fn startup_session_timeout(startup_probe_timeout: Duration) -> Duration {
    env_duration_ms(
        "TAKD_TOR_STARTUP_SESSION_TIMEOUT_MS",
        DEFAULT_STARTUP_SESSION_TIMEOUT_MS,
    )
    .min(startup_probe_timeout)
}

fn env_duration_ms(name: &str, default_ms: u64) -> Duration {
    Duration::from_millis(
        std::env::var(name)
            .ok()
            .and_then(|value| value.trim().parse::<u64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(default_ms),
    )
}
