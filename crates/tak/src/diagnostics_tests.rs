use anyhow::Error;
use tak_core::model::RemoteTransportKind;
use tak_exec::{NoMatchingRemoteError, RemoteCandidateDiagnostic, RemoteCandidateRejection};

use crate::diagnostics::{
    RenderMode, no_matching_remote_lines, render_error_report_with_mode, render_lines,
};

fn sample_error() -> NoMatchingRemoteError {
    NoMatchingRemoteError {
        task_label: "//:collect_remote_report".into(),
        required: tak_exec::RequiredRemoteDiagnostic {
            pool: Some("build".into()),
            required_tags: vec!["builder".into()],
            required_capabilities: vec!["linux".into()],
            transport_kind: RemoteTransportKind::Tor,
        },
        configured_remote_count: 1,
        enabled_remote_count: 1,
        enabled_remotes: vec![RemoteCandidateDiagnostic {
            node_id: "builder-default".into(),
            endpoint: "http://builder-default.onion".into(),
            pools: vec!["default".into()],
            tags: vec!["builder".into()],
            capabilities: vec!["linux".into()],
            transport: "tor".into(),
            rejection_reasons: vec![RemoteCandidateRejection::PoolMismatch {
                required: "build".into(),
                available: vec!["default".into()],
            }],
        }],
    }
}

#[test]
fn plain_rendering_omits_ansi_sequences() {
    let rendered = render_lines(
        &no_matching_remote_lines(&sample_error()),
        RenderMode::Plain,
    );
    assert!(rendered.contains("Remote placement failed for //:collect_remote_report"));
    assert!(!rendered.contains("\u{1b}["));
}

#[test]
fn ansi_rendering_emits_escape_sequences_for_structured_errors() {
    let rendered = render_lines(&no_matching_remote_lines(&sample_error()), RenderMode::Ansi);
    assert!(rendered.contains("\u{1b}["));
    assert!(rendered.contains("builder-default"));
}

#[test]
fn generic_errors_render_through_the_same_framework() {
    let rendered = render_error_report_with_mode(
        &Error::msg("coordination status is unavailable in this client-only build"),
        RenderMode::Plain,
    );
    assert!(
        rendered.contains("Error: coordination status is unavailable in this client-only build")
    );
    assert!(!rendered.contains("\u{1b}["));
}

#[test]
fn remote_node_info_timeouts_render_actionable_next_steps() {
    let rendered = render_error_report_with_mode(
        &Error::msg(
            "infra error: no reachable remote fallback candidates for task check: infra error: remote node builder-a at http://builder-a.onion via tor node info request timed out",
        ),
        RenderMode::Plain,
    );

    assert!(rendered.contains("Error: Remote node info probe timed out for check"));
    assert!(rendered.contains(
        "Tak could not get /v1/node/info from any candidate remote before the preflight timeout."
    ));
    assert!(rendered.contains("tak remote status"));
    assert!(rendered.contains("takd logs --lines 100"));
}
