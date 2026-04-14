use anyhow::Error;
use tak_exec::{
    RemoteObservation, RemotePreflightExhaustedError, RemotePreflightFailure,
    RemotePreflightFailureKind,
};

use crate::diagnostics::{RenderMode, render_error_report_with_mode};

#[test]
fn remote_node_info_timeouts_render_actionable_next_steps() {
    let rendered = render_error_report_with_mode(&Error::new(timeout_error()), RenderMode::Plain);

    assert!(rendered.contains("Error: Remote node info probe timed out for check"));
    assert!(rendered.contains("builder-a"));
    assert!(rendered.contains("recovering"));
    assert!(rendered.contains("rendezvous accept failed"));
}

#[test]
fn remote_preflight_http_failures_render_candidate_causes() {
    let rendered = render_error_report_with_mode(&Error::new(http_error()), RenderMode::Plain);

    assert!(rendered.contains("builder-auth"), "rendered:\n{rendered}");
    assert!(rendered.contains("HTTP 401"), "rendered:\n{rendered}");
    assert!(
        rendered.contains("builder-bad-proto"),
        "rendered:\n{rendered}"
    );
    assert!(
        rendered.contains("invalid protobuf"),
        "rendered:\n{rendered}"
    );
}

fn timeout_error() -> RemotePreflightExhaustedError {
    RemotePreflightExhaustedError {
        task_label: "check".into(),
        failures: vec![RemotePreflightFailure {
            node_id: "builder-a".into(),
            endpoint: "http://builder-a.onion".into(),
            transport: "tor".into(),
            kind: RemotePreflightFailureKind::Timeout,
            message: "node info request timed out".into(),
            live_transport_state: None,
            live_transport_detail: None,
            last_observation: Some(RemoteObservation {
                node_id: "builder-a".into(),
                sampled_at_ms: 1_734_000_000_000,
                transport: "tor".into(),
                healthy: false,
                transport_state: "recovering".into(),
                transport_detail: "rendezvous accept failed".into(),
            }),
        }],
    }
}

fn http_error() -> RemotePreflightExhaustedError {
    RemotePreflightExhaustedError {
        task_label: "check".into(),
        failures: vec![
            RemotePreflightFailure {
                node_id: "builder-auth".into(),
                endpoint: "http://builder-auth".into(),
                transport: "direct".into(),
                kind: RemotePreflightFailureKind::HttpStatus,
                message: "infra error: remote node builder-auth node info probe failed with HTTP 401".into(),
                live_transport_state: None,
                live_transport_detail: None,
                last_observation: None,
            },
            RemotePreflightFailure {
                node_id: "builder-bad-proto".into(),
                endpoint: "http://builder-bad-proto".into(),
                transport: "direct".into(),
                kind: RemotePreflightFailureKind::InvalidMetadata,
                message: "infra error: remote node builder-bad-proto returned invalid protobuf for node info".into(),
                live_transport_state: None,
                live_transport_detail: None,
                last_observation: None,
            },
        ],
    }
}
