#![cfg(test)]

use tor_hsservice::status::State;

use super::{
    SelfProbeRecoveryAction, format_arti_transport_detail, hidden_service_probe_gate,
    self_probe_failure_action,
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
    assert_eq!(
        self_probe_failure_action(
            "connect takd hidden-service startup probe: Unable to download hidden service descriptor"
        ),
        SelfProbeRecoveryAction::KeepWaiting
    );
    assert_eq!(
        self_probe_failure_action(
            "connect takd hidden-service startup probe: hidden-service circuit failed"
        ),
        SelfProbeRecoveryAction::KeepWaiting
    );
    assert_eq!(
        self_probe_failure_action("takd onion service request stream ended"),
        SelfProbeRecoveryAction::RelaunchService
    );
}

#[test]
fn startup_probe_timeout_on_descriptor_failures_restarts_tor_client() {
    let action = self_probe_failure_action(
        "Tor onion service at http://builder-a.onion did not become reachable within 60000ms during takd startup: \
         connect takd hidden-service startup probe: Unable to download hidden service descriptor",
    );

    assert_eq!(action, SelfProbeRecoveryAction::RestartTorClient);
}

#[test]
fn guard_exhaustion_restarts_tor_client() {
    let action = self_probe_failure_action(
        "connect takd hidden-service startup probe: Unable to select a guard relay: \
         No usable guards. Rejected 60/60 as down, then 0/0 as pending, then 0/0 as unsuitable to purpose, then 0/0 with filter.",
    );

    assert_eq!(action, SelfProbeRecoveryAction::RestartTorClient);
}

#[test]
fn single_descriptor_failure_keeps_waiting_for_readiness() {
    let action = self_probe_failure_action(
        "connect takd hidden-service startup probe: Unable to download hidden service descriptor",
    );

    assert_eq!(action, SelfProbeRecoveryAction::KeepWaiting);
}

#[test]
fn request_stream_end_relaunches_onion_service() {
    let action = self_probe_failure_action("takd onion service request stream ended");

    assert_eq!(action, SelfProbeRecoveryAction::RelaunchService);
}
