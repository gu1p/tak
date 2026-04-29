use tor_hsservice::status::State as OnionServiceState;

pub(super) struct HiddenServiceProbeGate {
    allows_probe: bool,
    requires_relaunch: bool,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(super) enum SelfProbeRecoveryAction {
    KeepWaiting,
    RelaunchService,
    RestartTorClient,
}

impl HiddenServiceProbeGate {
    pub(super) fn allows_probe(&self) -> bool {
        self.allows_probe
    }

    pub(super) fn requires_relaunch(&self) -> bool {
        self.requires_relaunch
    }
}

pub(super) fn hidden_service_probe_gate(state: OnionServiceState) -> HiddenServiceProbeGate {
    match state {
        OnionServiceState::Bootstrapping
        | OnionServiceState::DegradedReachable
        | OnionServiceState::Running => HiddenServiceProbeGate {
            allows_probe: true,
            requires_relaunch: false,
        },
        OnionServiceState::Broken => HiddenServiceProbeGate {
            allows_probe: false,
            requires_relaunch: true,
        },
        _ => HiddenServiceProbeGate {
            allows_probe: false,
            requires_relaunch: false,
        },
    }
}

pub(super) fn format_arti_transport_detail(
    base_url: &str,
    bootstrap_status: impl AsRef<str>,
    onion_state: OnionServiceState,
    problem: Option<&str>,
) -> String {
    let problem = problem
        .filter(|value| !value.trim().is_empty())
        .map(|value| format!("; problem={value}"))
        .unwrap_or_default();
    format!(
        "Arti onion-service state={onion_state:?}; Arti bootstrap: {}; base_url={base_url}{problem}",
        bootstrap_status.as_ref()
    )
}

pub(super) fn self_probe_failure_action(detail: &str) -> SelfProbeRecoveryAction {
    let detail = detail.to_ascii_lowercase();
    if detail.contains("request stream ended") {
        return SelfProbeRecoveryAction::RelaunchService;
    }
    if tor_guard_exhaustion_signal(&detail)
        || tor_fallback_exhaustion_signal(&detail)
        || startup_probe_timeout_exhausted(&detail) && tor_startup_failure_signal(&detail)
    {
        return SelfProbeRecoveryAction::RestartTorClient;
    }
    SelfProbeRecoveryAction::KeepWaiting
}

pub(super) fn tor_startup_failure_signal(detail: &str) -> bool {
    let detail = detail.to_ascii_lowercase();
    tor_guard_exhaustion_signal(&detail)
        || tor_fallback_exhaustion_signal(&detail)
        || detail.contains("unable to download hidden service descriptor")
        || detail.contains("failed to obtain hidden service descriptor")
        || detail.contains("hidden-service circuit")
        || detail.contains("failed to obtain hidden service circuit")
        || detail.contains("rendezvous circuit")
        || detail.contains("tried to find or build a tunnel")
}

pub(super) fn tor_guard_exhaustion_signal(detail: &str) -> bool {
    let detail = detail.to_ascii_lowercase();
    detail.contains("no usable guards") || detail.contains("unable to select a guard relay")
}

fn tor_fallback_exhaustion_signal(detail: &str) -> bool {
    detail.contains("no usable fallbacks")
}

fn startup_probe_timeout_exhausted(detail: &str) -> bool {
    detail.contains("did not become reachable within") && detail.contains("during takd startup")
}

#[path = "status_detail_tests.rs"]
mod status_detail_tests;
