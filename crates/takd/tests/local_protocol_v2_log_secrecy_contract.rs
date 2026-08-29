use crate::support::local_daemon_service::LocalDaemonService;

#[test]
fn local_protocol_failure_logs_redact_values_but_owner_response_keeps_cause() {
    let daemon = LocalDaemonService::start();

    daemon.exchange(
        r#"{"protocol_version":2,"request_id":"audit","operation":{"type":"GetRun","run_id":"TAK_LOG_SECRET_V2"}}"#,
    );
    daemon.exchange(r#"{"type":"TAK_LOG_SECRET_LEGACY","request_id":"audit"}"#);
    let legacy_failure = daemon.exchange(
        r#"{"type":"ForwardRemoteHttp","request_id":"TAK_LOG_SECRET_REQUEST","node_id":"TAK_LOG_SECRET_NODE","method":"GET","path":"/","headers":[],"body":[]}"#,
    );
    let legacy_failure: serde_json::Value =
        serde_json::from_str(&legacy_failure).expect("decode legacy failure response");
    assert_eq!(
        legacy_failure["message"], "unknown Tor peer TAK_LOG_SECRET_NODE",
        "owner response lost its diagnostic"
    );

    let log = daemon.service_log();
    for event in [
        "recognized v2 GetRun request",
        "invalid legacy local daemon request",
        "local daemon request failed",
    ] {
        assert!(log.contains(event), "missing protocol event: {event}");
    }
    for secret in [
        "TAK_LOG_SECRET_V2",
        "TAK_LOG_SECRET_LEGACY",
        "TAK_LOG_SECRET_REQUEST",
        "TAK_LOG_SECRET_NODE",
    ] {
        assert!(!log.contains(secret), "service log leaked {secret}");
    }
}
