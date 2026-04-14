use anyhow::Error;
use ratatui::text::{Line, Span};
use tak_exec::{RemotePreflightExhaustedError, RemotePreflightFailure, RemotePreflightFailureKind};

use super::{STYLE_ERROR, STYLE_SECTION};

pub(super) fn remote_preflight_lines(err: &Error) -> Option<Vec<Line<'static>>> {
    let error = err.downcast_ref::<RemotePreflightExhaustedError>()?;
    let timed_out = error
        .failures
        .iter()
        .any(|failure| failure.kind == RemotePreflightFailureKind::Timeout);
    let unhealthy_only = !timed_out
        && !error.failures.is_empty()
        && error
            .failures
            .iter()
            .all(|failure| failure.kind == RemotePreflightFailureKind::Unhealthy);
    let mut lines = vec![
        Line::from(vec![
            Span::styled("Error", STYLE_ERROR),
            Span::raw(": "),
            Span::raw(render_preflight_title(error, timed_out, unhealthy_only)),
        ]),
        Line::from(vec![
            Span::styled("What happened", STYLE_SECTION),
            Span::raw(": "),
            Span::raw(render_preflight_summary(timed_out, unhealthy_only)),
        ]),
    ];
    if let Some(line) = render_earlier_failure_line(err) {
        lines.push(line);
    }
    if !error.failures.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "Candidate failures:",
            STYLE_SECTION,
        )]));
        lines.extend(error.failures.iter().map(render_candidate_failure_line));
    }
    if error.failures.iter().any(has_transport_signal) {
        lines.push(Line::from(vec![Span::styled(
            "Remote state:",
            STYLE_SECTION,
        )]));
        lines.extend(
            error
                .failures
                .iter()
                .filter_map(render_transport_signal_line),
        );
    }
    lines.extend(render_next_checks(timed_out, unhealthy_only));
    Some(lines)
}

fn render_preflight_title(
    error: &RemotePreflightExhaustedError,
    timed_out: bool,
    unhealthy_only: bool,
) -> String {
    if timed_out {
        format!("Remote node info probe timed out for {}", error.task_label)
    } else if unhealthy_only {
        format!("Remote transport is not ready for {}", error.task_label)
    } else {
        format!("Remote preflight failed for {}", error.task_label)
    }
}

fn render_preflight_summary(timed_out: bool, unhealthy_only: bool) -> &'static str {
    if timed_out {
        "Tak could not get /v1/node/info from any candidate remote before the preflight timeout."
    } else if unhealthy_only {
        "Tak reached the remote node info endpoint, but every candidate reported a non-ready transport state."
    } else {
        "Tak could not complete /v1/node/info preflight against any candidate remote."
    }
}

fn render_earlier_failure_line(err: &Error) -> Option<Line<'static>> {
    let earlier = err
        .chain()
        .take_while(|cause| !cause.is::<RemotePreflightExhaustedError>())
        .map(std::string::ToString::to_string)
        .filter(|message| !message.is_empty())
        .collect::<Vec<_>>();
    (!earlier.is_empty()).then(|| {
        Line::from(vec![
            Span::styled("Earlier failure", STYLE_SECTION),
            Span::raw(": "),
            Span::raw(earlier.join(": ")),
        ])
    })
}

fn render_candidate_failure_line(failure: &RemotePreflightFailure) -> Line<'static> {
    Line::from(format!("  - {}: {}", failure.node_id, failure.message))
}

fn has_transport_signal(failure: &RemotePreflightFailure) -> bool {
    failure.live_transport_state.is_some() || failure.last_observation.is_some()
}

fn render_transport_signal_line(failure: &RemotePreflightFailure) -> Option<Line<'static>> {
    if let Some(state) = failure.live_transport_state.as_deref() {
        return Some(Line::from(format!(
            "  - {} state={} detail={}",
            failure.node_id,
            state,
            failure.live_transport_detail.as_deref().unwrap_or("n/a"),
        )));
    }
    let observation = failure.last_observation.as_ref()?;
    Some(Line::from(format!(
        "  - {} last_known_state={} detail={}",
        failure.node_id,
        observation.transport_state,
        if observation.transport_detail.is_empty() {
            "n/a"
        } else {
            observation.transport_detail.as_str()
        },
    )))
}

fn render_next_checks(timed_out: bool, unhealthy_only: bool) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(vec![Span::styled("Next checks:", STYLE_SECTION)]),
        Line::from("  tak remote status"),
    ];
    if timed_out || unhealthy_only {
        lines.extend([
            Line::from("  remote host: takd status"),
            Line::from("  remote host: takd logs --lines 100"),
            Line::from(
                "  if takd logs show Network unreachable, Host is unreachable, or Could not connect rendezvous circuit, treat it as remote Tor/network reachability",
            ),
        ]);
    } else {
        lines.extend([
            Line::from("  remote host: takd logs --lines 100"),
            Line::from("  verify bearer_token, base_url, and remote node version compatibility"),
        ]);
    }
    lines
}
