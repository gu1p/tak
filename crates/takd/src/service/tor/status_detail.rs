use tor_hsservice::status::State as OnionServiceState;

pub(super) struct HiddenServiceProbeGate {
    allows_probe: bool,
    requires_relaunch: bool,
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

pub(super) fn should_relaunch_for_self_probe_error(detail: &str) -> bool {
    let detail = detail.to_ascii_lowercase();
    if detail.contains("unable to download hidden service descriptor")
        || detail.contains("hidden-service circuit")
        || detail.contains("rendezvous circuit")
    {
        return false;
    }
    detail.contains("request stream ended")
}

#[path = "status_detail_tests.rs"]
mod status_detail_tests;
