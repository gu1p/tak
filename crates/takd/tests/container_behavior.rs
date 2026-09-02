//! Behavioral tests for deterministic container engine selection policy.

use takd::{
    ContainerEngine, ContainerEngineProbe, HostPlatform, select_container_engine_with_probe,
};

#[path = "container_behavior_errors.rs"]
mod errors;

#[derive(Debug)]
enum ProbeOutcome {
    Ok,
    Err(&'static str),
}

#[derive(Debug)]
struct FakeProbe {
    docker: ProbeOutcome,
    podman: ProbeOutcome,
    calls: Vec<ContainerEngine>,
}

impl FakeProbe {
    fn new(docker: ProbeOutcome, podman: ProbeOutcome) -> Self {
        Self {
            docker,
            podman,
            calls: Vec::new(),
        }
    }
}

impl ContainerEngineProbe for FakeProbe {
    fn probe(&mut self, engine: ContainerEngine) -> Result<(), String> {
        self.calls.push(engine);
        match engine {
            ContainerEngine::Docker => match self.docker {
                ProbeOutcome::Ok => Ok(()),
                ProbeOutcome::Err(message) => Err(message.to_string()),
            },
            ContainerEngine::Podman => match self.podman {
                ProbeOutcome::Ok => Ok(()),
                ProbeOutcome::Err(message) => Err(message.to_string()),
            },
        }
    }
}

#[test]
fn selects_docker_first_and_short_circuits_when_available() {
    let mut probe = FakeProbe::new(ProbeOutcome::Ok, ProbeOutcome::Ok);

    let selected = select_container_engine_with_probe(HostPlatform::MacOs, &mut probe)
        .expect("docker should be selected");

    assert_eq!(selected, ContainerEngine::Docker);
    assert_eq!(probe.calls, vec![ContainerEngine::Docker]);
}

#[test]
fn falls_back_to_podman_on_macos_when_docker_is_unavailable() {
    let mut probe = FakeProbe::new(ProbeOutcome::Err("docker unavailable"), ProbeOutcome::Ok);

    let selected = select_container_engine_with_probe(HostPlatform::MacOs, &mut probe)
        .expect("podman should be selected on macos fallback");

    assert_eq!(selected, ContainerEngine::Podman);
    assert_eq!(
        probe.calls,
        vec![ContainerEngine::Docker, ContainerEngine::Podman]
    );
}
