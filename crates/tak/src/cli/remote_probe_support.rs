use anyhow::Error;
use std::time::Duration;

const DEFAULT_TOR_PROBE_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_TOR_PROBE_BACKOFF: Duration = Duration::from_secs(1);

#[derive(Clone, Copy)]
pub(super) struct TorProbeRetryPolicy {
    pub(super) timeout: Duration,
    pub(super) backoff: Duration,
}

pub(super) enum ProbeAttemptError {
    Retryable(Error),
    Final(Error),
}

impl ProbeAttemptError {
    pub(super) fn retryable(err: Error) -> Self {
        Self::Retryable(err)
    }

    pub(super) fn final_error(err: Error) -> Self {
        Self::Final(err)
    }

    pub(super) fn is_retryable(&self) -> bool {
        matches!(self, Self::Retryable(_))
    }

    pub(super) fn into_anyhow(self) -> Error {
        match self {
            Self::Retryable(err) | Self::Final(err) => err,
        }
    }
}

pub(super) fn test_tor_onion_dial_addr() -> Option<String> {
    std::env::var("TAK_TEST_TOR_ONION_DIAL_ADDR")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(super) fn tor_probe_retry_policy() -> TorProbeRetryPolicy {
    TorProbeRetryPolicy {
        timeout: env_duration_ms("TAK_TEST_TOR_PROBE_TIMEOUT_MS", DEFAULT_TOR_PROBE_TIMEOUT),
        backoff: env_duration_ms("TAK_TEST_TOR_PROBE_BACKOFF_MS", DEFAULT_TOR_PROBE_BACKOFF),
    }
}

fn env_duration_ms(name: &str, default: Duration) -> Duration {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or(default)
}
