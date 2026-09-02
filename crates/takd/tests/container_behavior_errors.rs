use super::*;

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
