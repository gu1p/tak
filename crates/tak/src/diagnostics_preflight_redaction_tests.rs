use anyhow::Error;
use tak_exec::{
    RemoteObservation, RemotePreflightExhaustedError, RemotePreflightFailure,
    RemotePreflightFailureKind,
};

use crate::diagnostics::{RenderMode, render_error_report_with_mode};

#[test]
fn remote_preflight_shows_onion_urls_in_failure_messages_and_transport_detail() {
    let rendered =
        render_error_report_with_mode(&Error::new(onion_detail_error()), RenderMode::Plain);

    assert!(rendered.contains(
        "builder-live: infra error: remote node builder-live reported transport state recovering at http://builder-live.onion"
    ));
    assert!(rendered.contains(
        "builder-last: infra error: remote node builder-last at http://builder-last.onion via tor node info request timed out"
    ));
    assert!(rendered.contains(
        "detail=Tor onion service at http://builder-live.onion did not become reachable within 1000ms during takd startup"
    ));
    assert!(rendered.contains("detail=malformed HTTP response from http://builder-last.onion"));
}

fn onion_detail_error() -> RemotePreflightExhaustedError {
    RemotePreflightExhaustedError {
        task_label: "check".into(),
        failures: vec![
            RemotePreflightFailure {
                node_id: "builder-live".into(),
                endpoint: "http://builder-live.onion".into(),
                transport: "tor".into(),
                kind: RemotePreflightFailureKind::Unhealthy,
                message: "infra error: remote node builder-live reported transport state recovering at http://builder-live.onion".into(),
                live_transport_state: Some("recovering".into()),
                live_transport_detail: Some(
                    "Tor onion service at http://builder-live.onion did not become reachable within 1000ms during takd startup".into(),
                ),
                last_observation: None,
            },
            RemotePreflightFailure {
                node_id: "builder-last".into(),
                endpoint: "http://builder-last.onion".into(),
                transport: "tor".into(),
                kind: RemotePreflightFailureKind::Timeout,
                message: "infra error: remote node builder-last at http://builder-last.onion via tor node info request timed out".into(),
                live_transport_state: None,
                live_transport_detail: None,
                last_observation: Some(RemoteObservation {
                    node_id: "builder-last".into(),
                    sampled_at_ms: 1_734_000_000_000,
                    transport: "tor".into(),
                    healthy: false,
                    transport_state: "recovering".into(),
                    transport_detail: "malformed HTTP response from http://builder-last.onion".into(),
                }),
            },
        ],
    }
}
