mod remote_timeout;
mod render;

use std::io::IsTerminal;

use anyhow::Error;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use tak_exec::{NoMatchingRemoteError, RemoteCandidateDiagnostic, RemoteCandidateRejection};

use remote_timeout::remote_node_info_timeout_lines;
pub(crate) use render::render_lines;

const STYLE_ERROR: Style = Style::new().fg(Color::Red).add_modifier(Modifier::BOLD);
const STYLE_SECTION: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);
const STYLE_DIM: Style = Style::new().fg(Color::Gray).add_modifier(Modifier::DIM);
const STYLE_WARN: Style = Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD);

pub fn render_error_report(err: &Error) -> String {
    render_error_report_with_mode(err, default_render_mode())
}

fn default_render_mode() -> RenderMode {
    if std::env::var_os("NO_COLOR").is_some() || !std::io::stderr().is_terminal() {
        RenderMode::Plain
    } else {
        RenderMode::Ansi
    }
}

pub(crate) fn render_error_report_with_mode(err: &Error, mode: RenderMode) -> String {
    if let Some(no_match) = err.downcast_ref::<NoMatchingRemoteError>() {
        return render_lines(&no_matching_remote_lines(no_match), mode);
    }
    if let Some(lines) = remote_node_info_timeout_lines(err) {
        return render_lines(&lines, mode);
    }
    render_lines(&generic_error_lines(err), mode)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RenderMode {
    Plain,
    Ansi,
}

pub(crate) fn no_matching_remote_lines(err: &NoMatchingRemoteError) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(vec![
            Span::styled("Error", STYLE_ERROR),
            Span::raw(": "),
            Span::raw(format!("Remote placement failed for {}", err.task_label)),
        ]),
        Line::from(vec![
            Span::styled("Required remote", STYLE_SECTION),
            Span::raw(": "),
            Span::raw(render_required_remote(err)),
        ]),
    ];

    if err.configured_remote_count == 0 {
        lines.push(Line::from(vec![
            Span::styled("Configured remotes", STYLE_SECTION),
            Span::raw(": none"),
        ]));
        return lines;
    }

    if err.enabled_remote_count == 0 {
        lines.push(Line::from(vec![
            Span::styled("Configured remotes", STYLE_SECTION),
            Span::raw(format!(": {}", err.configured_remote_count)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Enabled remotes", STYLE_SECTION),
            Span::raw(": none"),
        ]));
        return lines;
    }

    lines.push(Line::from(vec![Span::styled(
        "Enabled remotes:",
        STYLE_SECTION,
    )]));
    for remote in &err.enabled_remotes {
        lines.push(remote_summary_line(remote));
        lines.push(remote_rejection_line(remote));
    }
    lines
}

fn render_required_remote(err: &NoMatchingRemoteError) -> String {
    let mut fields = Vec::new();
    if let Some(pool) = err.required.pool.as_deref() {
        fields.push(format!("pool={pool}"));
    }
    if !err.required.required_tags.is_empty() {
        fields.push(format!("tags={}", err.required.required_tags.join(",")));
    }
    if !err.required.required_capabilities.is_empty() {
        fields.push(format!(
            "capabilities={}",
            err.required.required_capabilities.join(",")
        ));
    }
    fields.push(format!(
        "transport={}",
        err.required.transport_kind.as_result_value()
    ));
    fields.join(" ")
}

fn remote_summary_line(remote: &RemoteCandidateDiagnostic) -> Line<'static> {
    Line::from(vec![
        Span::raw("  - "),
        Span::styled(remote.node_id.clone(), STYLE_WARN),
        Span::raw(" "),
        Span::styled("endpoint=", STYLE_DIM),
        Span::raw(remote.endpoint.clone()),
        Span::raw(" "),
        Span::styled("pools=", STYLE_DIM),
        Span::raw(render_list(&remote.pools)),
        Span::raw(" "),
        Span::styled("tags=", STYLE_DIM),
        Span::raw(render_list(&remote.tags)),
        Span::raw(" "),
        Span::styled("capabilities=", STYLE_DIM),
        Span::raw(render_list(&remote.capabilities)),
        Span::raw(" "),
        Span::styled("transport=", STYLE_DIM),
        Span::raw(remote.transport.clone()),
    ])
}

fn remote_rejection_line(remote: &RemoteCandidateDiagnostic) -> Line<'static> {
    let reasons = remote
        .rejection_reasons
        .iter()
        .map(render_rejection_reason)
        .collect::<Vec<_>>()
        .join("; ");
    Line::from(vec![
        Span::raw("    "),
        Span::styled("rejected", STYLE_WARN),
        Span::raw(": "),
        Span::raw(reasons),
    ])
}

fn render_rejection_reason(reason: &RemoteCandidateRejection) -> String {
    match reason {
        RemoteCandidateRejection::PoolMismatch {
            required,
            available,
        } => format!(
            "pool mismatch: required {required}, remote pools={}",
            render_list(available)
        ),
        RemoteCandidateRejection::MissingTags { missing, .. } => {
            format!("missing tags={}", missing.join(","))
        }
        RemoteCandidateRejection::MissingCapabilities { missing, .. } => {
            format!("missing capabilities={}", missing.join(","))
        }
        RemoteCandidateRejection::TransportMismatch {
            required,
            available,
        } => format!(
            "transport mismatch: required {}, remote transport={available}",
            required.as_result_value()
        ),
    }
}

fn render_list(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values.join(",")
    }
}

fn remote_node_info_timeout_lines(err: &Error) -> Option<Vec<Line<'static>>> {
    let rendered = format!("{err:#}");
    if !rendered.contains("no reachable remote fallback candidates for task ")
        || !rendered.contains("node info request timed out")
    {
        return None;
    }

    let task_label = extract_timeout_task_label(&rendered).unwrap_or("requested task");
    Some(vec![
        Line::from(vec![
            Span::styled("Error", STYLE_ERROR),
            Span::raw(": "),
            Span::raw(format!("Remote node info probe timed out for {task_label}")),
        ]),
        Line::from(vec![
            Span::styled("What happened", STYLE_SECTION),
            Span::raw(": "),
            Span::raw(
                "Tak could not get /v1/node/info from any candidate remote before the preflight timeout.",
            ),
        ]),
        Line::from(vec![Span::styled("Next checks:", STYLE_SECTION)]),
        Line::from("  tak remote status"),
        Line::from("  remote host: takd status"),
        Line::from("  remote host: takd logs --lines 100"),
        Line::from(
            "  if takd logs show Network unreachable, Host is unreachable, or Could not connect rendezvous circuit, treat it as remote Tor/network reachability",
        ),
        Line::from(vec![
            Span::styled("Original error", STYLE_SECTION),
            Span::raw(": "),
            Span::raw(rendered.lines().next().unwrap_or("unknown error").to_string()),
        ]),
    ])
}

fn extract_timeout_task_label(rendered: &str) -> Option<&str> {
    let marker = "no reachable remote fallback candidates for task ";
    let start = rendered.find(marker)? + marker.len();
    let remainder = &rendered[start..];
    let end = remainder.find(':')?;
    Some(remainder[..end].trim())
}

fn generic_error_lines(err: &Error) -> Vec<Line<'static>> {
    let rendered = format!("{err:#}");
    let mut parts = rendered.lines();
    let first = parts.next().unwrap_or("unknown error").to_string();
    let mut lines = vec![Line::from(vec![
        Span::styled("Error", STYLE_ERROR),
        Span::raw(": "),
        Span::raw(first),
    ])];
    lines.extend(parts.map(|line| Line::from(line.to_string())));
    lines
}
