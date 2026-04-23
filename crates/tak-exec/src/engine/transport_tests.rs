#![cfg(test)]

use std::time::Duration;

use super::StrictRemoteTarget;
use super::remote_models::StrictRemoteTransportKind;
use super::transport::{phase_timeout, preflight_timeout};
use super::transport_tor::tor_connect_timeout;

#[test]
fn direct_preflight_timeout_stays_at_one_second() {
    let target = target(StrictRemoteTransportKind::Direct);

    assert_eq!(preflight_timeout(&target), Duration::from_secs(1));
}

#[test]
fn tor_preflight_timeout_uses_tor_connect_timeout() {
    let target = target(StrictRemoteTransportKind::Tor);

    assert_eq!(preflight_timeout(&target), tor_connect_timeout());
}

#[test]
fn direct_phase_timeout_keeps_requested_value() {
    let target = target(StrictRemoteTransportKind::Direct);
    let requested = Duration::from_millis(250);

    assert_eq!(phase_timeout(&target, requested), requested);
}

#[test]
fn tor_phase_timeout_applies_tor_minimum() {
    let target = target(StrictRemoteTransportKind::Tor);
    let requested = Duration::from_millis(250);

    assert_eq!(phase_timeout(&target, requested), tor_connect_timeout());
}

fn target(transport_kind: StrictRemoteTransportKind) -> StrictRemoteTarget {
    StrictRemoteTarget {
        node_id: "builder-a".into(),
        endpoint: "http://127.0.0.1:8080".into(),
        transport_kind,
        bearer_token: "secret".into(),
        runtime: None,
    }
}
