use std::time::Duration;

use anyhow::Error;

use super::super::status_detail::{tor_guard_exhaustion_signal, tor_startup_failure_signal};

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
