use crate::support::local_daemon_service::LocalDaemonService;

#[test]
fn local_protocol_failure_logs_redact_values_and_v1_execution_gets_upgrade_guidance() {
    let daemon = LocalDaemonService::start();

    daemon.exchange(
        r#"{"protocol_version":2,"request_id":"audit","operation":{"type":"GetRun","run_id":"TAK_LOG_SECRET_V2"}}"#,
    );
    let outputs = daemon.exchange(
        r#"{"protocol_version":2,"request_id":"outputs-audit","operation":{"type":"GetOutputManifest","run_id":"TAK_LOG_SECRET_OUTPUTS"}}"#,
    );
    let outputs: serde_json::Value =
        serde_json::from_str(&outputs).expect("decode outputs response");
    assert_eq!(outputs["request_id"], "outputs-audit");
    assert_eq!(outputs["code"], "run_not_found");
    daemon.exchange(r#"{"type":"TAK_LOG_SECRET_LEGACY","request_id":"audit"}"#);
    let v1_rejection = daemon.exchange(
        r#"{"type":"ForwardRemoteHttp","request_id":"TAK_LOG_SECRET_REQUEST","node_id":"TAK_LOG_SECRET_NODE","method":"GET","path":"/","headers":[],"body":[]}"#,
    );
    let v1_rejection: serde_json::Value =
        serde_json::from_str(&v1_rejection).expect("decode v1 rejection response");
    assert_eq!(
        v1_rejection["message"],
        "This takd requires protocol v2. Upgrade tak, takd, and workers together."
    );
    assert_eq!(v1_rejection["code"], "protocol_version_unsupported");

    let log = daemon.service_log();
    assert!(
        log.contains("invalid legacy local daemon request"),
        "missing invalid-request protocol event"
    );
    for secret in [
        "TAK_LOG_SECRET_V2",
        "TAK_LOG_SECRET_OUTPUTS",
        "TAK_LOG_SECRET_LEGACY",
        "TAK_LOG_SECRET_REQUEST",
        "TAK_LOG_SECRET_NODE",
    ] {
        assert!(!log.contains(secret), "service log leaked {secret}");
    }
}
