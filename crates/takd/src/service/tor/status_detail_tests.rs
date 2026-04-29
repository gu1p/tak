#![cfg(test)]

use tor_hsservice::status::State;

use super::{
    format_arti_transport_detail, hidden_service_probe_gate, should_relaunch_for_self_probe_error,
};

#[test]
fn arti_observations_format_as_clear_transport_detail() {
    let detail = format_arti_transport_detail(
        "http://builder-a.onion",
        "100%: Tor client is ready",
        State::Recovering,
        Some("descriptor upload timed out"),
    );

    assert!(detail.contains("http://builder-a.onion"));
    assert!(detail.contains("Arti bootstrap: 100%: Tor client is ready"));
    assert!(detail.contains("Arti onion-service state=Recovering"));
    assert!(detail.contains("problem=descriptor upload timed out"));
}

#[test]
fn hidden_service_status_controls_when_self_probe_should_run() {
    assert!(hidden_service_probe_gate(State::Running).allows_probe());
    assert!(hidden_service_probe_gate(State::DegradedReachable).allows_probe());
    assert!(hidden_service_probe_gate(State::Bootstrapping).allows_probe());
    assert!(!hidden_service_probe_gate(State::Recovering).allows_probe());
    assert!(!hidden_service_probe_gate(State::DegradedUnreachable).allows_probe());
    assert!(hidden_service_probe_gate(State::Broken).requires_relaunch());
}

#[test]
fn descriptor_download_failures_do_not_trigger_startup_relaunch() {
    assert!(!should_relaunch_for_self_probe_error(
        "connect takd hidden-service startup probe: Unable to download hidden service descriptor"
    ));
    assert!(!should_relaunch_for_self_probe_error(
        "connect takd hidden-service startup probe: hidden-service circuit failed"
    ));
    assert!(should_relaunch_for_self_probe_error(
        "takd onion service request stream ended"
    ));
}
