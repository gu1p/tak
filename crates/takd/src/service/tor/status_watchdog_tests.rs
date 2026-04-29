#![cfg(test)]

use tor_hsservice::status::State;

use super::{SelfProbeRecoveryAction, startup_watchdog_action};

#[test]
fn startup_watchdog_restarts_tor_client_when_arti_state_never_allows_probe() {
    assert_eq!(
        startup_watchdog_action(State::Recovering),
        SelfProbeRecoveryAction::RestartTorClient
    );
    assert_eq!(
        startup_watchdog_action(State::DegradedUnreachable),
        SelfProbeRecoveryAction::RestartTorClient
    );
    assert_eq!(
        startup_watchdog_action(State::Broken),
        SelfProbeRecoveryAction::RelaunchService
    );
    assert_eq!(
        startup_watchdog_action(State::Bootstrapping),
        SelfProbeRecoveryAction::KeepWaiting
    );
}
