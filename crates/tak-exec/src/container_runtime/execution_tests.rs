use super::execution::diagnostics::exit_137_diagnostic_message;

#[test]
fn exit_137_diagnostic_uses_engine_evidence_without_inventing_a_cause() {
    let confirmed = exit_137_diagnostic_message(Some(true), None);
    assert!(confirmed.contains("OOMKilled=true"));
    assert!(confirmed.contains("container OOM confirmed"));

    let disproved = exit_137_diagnostic_message(Some(false), None);
    assert!(disproved.contains("OOMKilled=false"));
    assert!(disproved.contains("cause is unknown"));
    assert!(!disproved.contains("host-level SIGKILL"));
    assert!(!disproved.contains("kernel OOM"));
    assert!(!disproved.contains("systemd-oomd"));

    let unavailable = exit_137_diagnostic_message(None, None);
    assert!(unavailable.contains("OOMKilled=unknown"));
    assert!(unavailable.contains("cause is unknown"));
}
