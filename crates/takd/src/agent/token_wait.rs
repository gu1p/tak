use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Result;
use tak_proto::decode_tor_invite;

use super::token_state::{
    ReadTokenError, read_token_error_into_anyhow, read_token_state, should_retry_token_error,
};
use super::{TransportHealth, TransportState, read_transport_health};

pub fn read_token_wait(state_root: &Path, timeout_secs: u64) -> Result<String> {
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        match read_ready_token_state(state_root) {
            Ok(token) => return Ok(token),
            Err(err) if should_retry_token_error(&err) && Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(100));
            }
            Err(err) => return Err(read_token_error_into_anyhow(err)),
        }
    }
}

fn read_ready_token_state(state_root: &Path) -> std::result::Result<String, ReadTokenError> {
    read_ready_token_once(state_root)
}

pub(super) fn read_ready_token_once(
    state_root: &Path,
) -> std::result::Result<String, ReadTokenError> {
    let token = read_token_state(state_root)?;
    require_current_transport_readiness(state_root, &token)?;
    Ok(token)
}

pub(super) fn read_token_once_unless_transport_is_reported_unready(
    state_root: &Path,
) -> std::result::Result<String, ReadTokenError> {
    let token = read_token_state(state_root)?;
    require_reported_transport_is_not_unready(state_root, &token)?;
    Ok(token)
}

fn require_current_transport_readiness(
    state_root: &Path,
    token: &str,
) -> std::result::Result<(), ReadTokenError> {
    let Ok(base_url) = decode_tor_invite(token) else {
        return Ok(());
    };
    let health = read_transport_health(state_root).map_err(ReadTokenError::Invalid)?;
    match health {
        Some(health)
            if health.transport_state == TransportState::Ready
                && health.base_url.as_deref() == Some(base_url.as_str()) =>
        {
            Ok(())
        }
        Some(health) => Err(ReadTokenError::TransportNotReady(
            tor_transport_wait_detail(&base_url, &health),
        )),
        None => Err(ReadTokenError::TransportNotReady(format!(
            "tor transport has not reported readiness for {base_url}"
        ))),
    }
}

fn require_reported_transport_is_not_unready(
    state_root: &Path,
    token: &str,
) -> std::result::Result<(), ReadTokenError> {
    let Ok(base_url) = decode_tor_invite(token) else {
        return Ok(());
    };
    let Some(health) = read_transport_health(state_root).map_err(ReadTokenError::Invalid)? else {
        return Ok(());
    };
    if health.transport_state == TransportState::Ready
        && health.base_url.as_deref() == Some(base_url.as_str())
    {
        return Ok(());
    }
    Err(ReadTokenError::TransportNotReady(
        tor_transport_wait_detail(&base_url, &health),
    ))
}

fn tor_transport_wait_detail(base_url: &str, health: &TransportHealth) -> String {
    let state = health.transport_state.as_str();
    let detail = health
        .detail
        .as_deref()
        .filter(|detail| !detail.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| fallback_transport_detail(health).to_string());
    match health.base_url.as_deref() {
        Some(current) if current != base_url => {
            format!("tor transport is {state} at {current}; token points to {base_url}: {detail}")
        }
        _ => format!("tor transport is {state} for {base_url}: {detail}"),
    }
}

fn fallback_transport_detail(health: &TransportHealth) -> &'static str {
    match health.transport_state {
        TransportState::Pending => {
            "waiting for takd Tor startup probe to report a readiness detail"
        }
        TransportState::Ready | TransportState::Recovering => "transport detail not reported",
    }
}
