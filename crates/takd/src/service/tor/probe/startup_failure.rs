use std::time::Duration;

use anyhow::Error;

use super::super::status_detail::{
    immediate_tor_startup_restart_signal, tor_guard_exhaustion_signal, tor_startup_failure_signal,
};

#[derive(Debug, Eq, PartialEq)]
pub(super) enum StartupTorFailureDecision {
    KeepWaiting,
    RestartTorClient { reason: String },
}

#[derive(Debug)]
pub(super) struct StartupTorFailureTracker {
    failure_threshold: u32,
    consecutive_tor_failures: u32,
    observed_tor_failure: Option<String>,
}

impl StartupTorFailureTracker {
    pub(super) fn new(failure_threshold: u32) -> Self {
        Self {
            failure_threshold: failure_threshold.max(1),
            consecutive_tor_failures: 0,
            observed_tor_failure: None,
        }
    }

    pub(super) fn record_failure(&mut self, detail: &str) -> StartupTorFailureDecision {
        if immediate_tor_startup_restart_signal(detail) {
            remember_tor_startup_failure_signal(&mut self.observed_tor_failure, detail);
            return StartupTorFailureDecision::RestartTorClient {
                reason: detail.to_string(),
            };
        }
        if !tor_startup_failure_signal(detail) {
            self.consecutive_tor_failures = 0;
            return StartupTorFailureDecision::KeepWaiting;
        }
        remember_tor_startup_failure_signal(&mut self.observed_tor_failure, detail);
        self.consecutive_tor_failures = self.consecutive_tor_failures.saturating_add(1);
        if self.consecutive_tor_failures >= self.failure_threshold {
            return StartupTorFailureDecision::RestartTorClient {
                reason: format!(
                    "{} consecutive Tor startup probe failures: {}",
                    self.consecutive_tor_failures,
                    self.observed_tor_failure.as_deref().unwrap_or(detail)
                ),
            };
        }
        StartupTorFailureDecision::KeepWaiting
    }

    pub(super) fn observed_tor_failure(&self) -> Option<&str> {
        self.observed_tor_failure.as_deref()
    }
}

#[derive(Debug)]
pub(super) struct StartupTorClientRestart {
    detail: String,
}

impl StartupTorClientRestart {
    pub(super) fn new(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
        }
    }
}

impl std::fmt::Display for StartupTorClientRestart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "embedded Arti client restart required during takd startup: {}",
            self.detail
        )
    }
}

impl std::error::Error for StartupTorClientRestart {}

pub(super) fn remember_tor_startup_failure_signal(observed: &mut Option<String>, detail: &str) {
    if !tor_startup_failure_signal(detail) {
        return;
    }
    if observed.is_none()
        || tor_guard_exhaustion_signal(detail)
            && !tor_guard_exhaustion_signal(observed.as_deref().unwrap_or_default())
    {
        *observed = Some(detail.to_string());
    }
}

pub(super) fn startup_probe_error(
    last_error: Error,
    observed_tor_failure: Option<&str>,
    base_url: &str,
    timeout: Duration,
) -> Error {
    let error = match observed_tor_failure {
        Some(detail) => last_error.context(format!("earlier Tor startup probe failure: {detail}")),
        None => last_error,
    };
    error.context(format!(
        "Tor onion service at {base_url} did not become reachable within {}ms during takd startup",
        timeout.as_millis()
    ))
}

#[path = "startup_failure_tests.rs"]
mod startup_failure_tests;
