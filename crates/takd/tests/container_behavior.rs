//! Behavioral tests for deterministic container engine selection policy.

use takd::{
    ContainerEngine, ContainerEngineProbe, HostPlatform, select_container_engine_with_probe,
};

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

#[test]
fn non_macos_does_not_fallback_to_podman_when_docker_is_unavailable() {
    let mut probe = FakeProbe::new(ProbeOutcome::Err("docker unavailable"), ProbeOutcome::Ok);

    let error = select_container_engine_with_probe(HostPlatform::Other, &mut probe)
        .expect_err("non-macos should not fallback to podman");

    assert_eq!(probe.calls, vec![ContainerEngine::Docker]);
    assert!(error.to_string().contains("attempted probes: docker"));
}

#[test]
fn error_lists_attempted_engines_without_leaking_probe_details() {
    let mut probe = FakeProbe::new(
        ProbeOutcome::Err("docker failed at /var/run/docker.sock token=secret-one"),
        ProbeOutcome::Err("podman failed at /usr/local/bin/podman token=secret-two"),
    );

    let error = select_container_engine_with_probe(HostPlatform::MacOs, &mut probe)
        .expect_err("both engines unavailable should return an infra error");
    let message = error.to_string();

    assert_eq!(
        probe.calls,
        vec![ContainerEngine::Docker, ContainerEngine::Podman]
    );
    assert!(message.contains("attempted probes: docker, podman"));
    assert!(!message.contains("/var/run/docker.sock"));
    assert!(!message.contains("/usr/local/bin/podman"));
    assert!(!message.contains("secret-one"));
    assert!(!message.contains("secret-two"));
}
