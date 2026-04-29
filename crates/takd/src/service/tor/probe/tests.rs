use std::time::{Duration, Instant};

use anyhow::Result;
use takd::agent::{TransportState, read_transport_health};
use tokio::time::sleep;

use super::{
    endpoint_host_port, endpoint_socket_addr, record_probe_failure, run_with_attempt_timeout,
};

#[tokio::test]
async fn attempt_timeout_caps_long_running_probe_steps() {
    let err = run_with_attempt_timeout(
        Instant::now() + Duration::from_millis(40),
        Duration::from_millis(10),
        "connect startup probe",
        async {
            sleep(Duration::from_millis(20)).await;
            Ok::<(), anyhow::Error>(())
        },
    )
    .await
    .expect_err("long probe steps should time out");
    assert!(format!("{err:#}").contains("connect startup probe timed out after 10ms"));
}

#[tokio::test]
async fn attempt_timeout_respects_remaining_deadline() {
    let err = run_with_attempt_timeout(
        Instant::now() + Duration::from_millis(6),
        Duration::from_millis(10),
        "read startup probe",
        async {
            sleep(Duration::from_millis(20)).await;
            Ok::<(), anyhow::Error>(())
        },
    )
    .await
    .expect_err("attempt timeout should clamp to the overall deadline");
    let message = format!("{err:#}");
    let timeout_ms = message
        .rsplit_once("after ")
        .and_then(|(_, value)| value.strip_suffix("ms"))
        .and_then(|value| value.parse::<u64>().ok())
        .expect("timeout message should include milliseconds");
    assert!(
        timeout_ms < 10,
        "expected remaining deadline to clamp below the 10ms cap, got: {message}"
    );
}

#[tokio::test]
async fn attempt_timeout_returns_successful_results() -> Result<()> {
    let value = run_with_attempt_timeout(
        Instant::now() + Duration::from_millis(40),
        Duration::from_millis(10),
        "probe startup endpoint",
        async { Ok::<_, anyhow::Error>(42_u8) },
    )
    .await?;
    assert_eq!(value, 42);
    Ok(())
}

#[tokio::test]
async fn attempt_timeout_rejects_expired_deadlines_before_starting() {
    let err = run_with_attempt_timeout(
        Instant::now(),
        Duration::from_millis(10),
        "startup probe",
        async { Ok::<(), anyhow::Error>(()) },
    )
    .await
    .expect_err("expired deadlines should fail before running");
    assert!(format!("{err:#}").contains("startup probe timed out before the attempt started"));
}

#[test]
fn endpoint_helpers_normalize_authority_and_validate_ports() {
    assert_eq!(
        endpoint_socket_addr("http://builder.example/path").expect("http default port"),
        "builder.example:80"
    );
    assert_eq!(
        endpoint_socket_addr("https://builder.example").expect("https default port"),
        "builder.example:443"
    );
    assert_eq!(
        endpoint_host_port("http://builder.example:91").expect("explicit port"),
        ("builder.example".to_string(), 91)
    );
    assert!(endpoint_socket_addr("builder.example").is_err());
    assert!(endpoint_host_port("http://builder.example:bad").is_err());
}

#[test]
fn endpoint_helpers_support_ipv6_and_strip_userinfo() {
    assert_eq!(
        endpoint_socket_addr("https://[::1]").expect("https ipv6"),
        "[::1]:443"
    );
    assert_eq!(
        endpoint_host_port("https://[::1]:8443/path").expect("ipv6 host port"),
        ("::1".to_string(), 8443)
    );
    assert_eq!(
        endpoint_socket_addr("https://user:pass@[::1]:9443/status").expect("userinfo stripped"),
        "[::1]:9443"
    );
}

#[test]
fn probe_failure_persists_pending_transport_detail() {
    let temp = tempfile::tempdir().expect("tempdir");

    record_probe_failure(
        Some(temp.path()),
        "http://builder-a.onion",
        "self-probe connect",
        7,
        Instant::now(),
        Duration::from_secs(60),
        "connect takd hidden-service startup probe: rendezvous circuit timed out",
    );

    let health = read_transport_health(temp.path())
        .expect("read health")
        .expect("health should exist");
    assert_eq!(health.transport_state, TransportState::Pending);
    assert_eq!(health.base_url.as_deref(), Some("http://builder-a.onion"));
    let detail = health.detail.expect("detail");
    assert!(detail.contains("self-probe connect attempt 7"));
    assert!(detail.contains("rendezvous circuit timed out"));
}
