use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use super::super::super::RemoteStatusResult;
use super::super::super::view::{RemoteStatusRow, RemoteStatusView};
use super::super::{
    age_since, format_cpu, format_image_cache, format_memory, format_needs, format_storage,
    human_bytes,
};
use super::{enabled_style, title_style};

const STYLE_OK: Style = Style::new().fg(Color::Green).add_modifier(Modifier::BOLD);
const STYLE_WARN: Style = Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD);
const STYLE_ERROR: Style = Style::new().fg(Color::Red).add_modifier(Modifier::BOLD);
const STYLE_CHECKING: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);

pub(super) fn node_line(
    index: usize,
    row: &RemoteStatusRow,
    tick: usize,
    color_enabled: bool,
) -> Line<'static> {
    let summary = row_summary(row);
    let style = enabled_style(summary.style, color_enabled);
    Line::from(vec![
        Span::styled(progress_bar(row, tick.saturating_add(index)), style),
        Span::raw(" "),
        Span::styled(format!("{:<8}", summary.badge), style),
        Span::raw(" "),
        Span::styled(
            format!("{:<18}", row.remote().node_id),
            title_style(color_enabled),
        ),
        Span::raw(" "),
        Span::raw(summary.detail),
    ])
}

pub(super) fn push_job_lines(lines: &mut Vec<Line<'static>>, view: &RemoteStatusView) {
    let mut any_jobs = false;
    for result in view.completed_results() {
        let Some(status) = &result.status else {
            continue;
        };
        for job in &status.active_jobs {
            any_jobs = true;
            lines.push(Line::from(format!(
                "{} {} attempt={} age={} needs={} exec_root={} runtime={}{}{}",
                result.remote.node_id,
                job.task_label,
                job.attempt,
                age_since(job.started_at_ms),
                format_needs(&job.needs),
                human_bytes(job.execution_root_bytes),
                job.runtime.as_deref().unwrap_or("none"),
                optional_field(" command=", job.command.as_deref()),
                optional_field(" source=", job.runtime_source.as_deref()),
            )));
        }
    }
    if !any_jobs {
        lines.push(Line::from("(none)"));
    }
}

fn row_summary(row: &RemoteStatusRow) -> RowSummary {
    match row {
        RemoteStatusRow::Checking { remote } => RowSummary {
            badge: "CHECKING",
            style: STYLE_CHECKING,
            detail: format!("{} probing /v1/node/status", remote.transport),
        },
        RemoteStatusRow::Complete(result) => complete_row_summary(result),
    }
}

fn complete_row_summary(result: &RemoteStatusResult) -> RowSummary {
    let transport = result
        .status
        .as_ref()
        .and_then(|status| status.node.as_ref().map(|node| node.transport.as_str()))
        .unwrap_or(result.remote.transport.as_str());
    let Some(status) = &result.status else {
        return RowSummary {
            badge: "ERROR",
            style: STYLE_ERROR,
            detail: format!(
                "{} {}",
                transport,
                result.error.as_deref().unwrap_or("unknown_error")
            ),
        };
    };

    let state = status
        .node
        .as_ref()
        .map(|node| node.transport_state.as_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("ready");
    let detail = status
        .node
        .as_ref()
        .map(|node| node.transport_detail.as_str())
        .filter(|value| !value.is_empty())
        .map(|value| format!(" detail={value}"))
        .unwrap_or_default();
    let (badge, style) = status_badge(status, state);

    RowSummary {
        badge,
        style,
        detail: format!(
            "{} state={} jobs={} cpu={} ram={} storage={} tak_exec={} image_cache={}{}",
            transport,
            state,
            status.active_jobs.len(),
            format_cpu(status.cpu.as_ref()),
            format_memory(status.memory.as_ref()),
            format_storage(status.storage.as_ref()),
            status
                .storage
                .as_ref()
                .map(|value| human_bytes(value.tak_execution_bytes))
                .unwrap_or_else(|| "n/a".to_string()),
            format_image_cache(status.image_cache.as_ref()),
            detail,
        ),
    }
}

fn status_badge(status: &tak_proto::NodeStatusResponse, state: &str) -> (&'static str, Style) {
    if !status.active_jobs.is_empty() {
        return ("BUSY", STYLE_WARN);
    }
    if state == "ready" && status.node.as_ref().is_none_or(|node| node.healthy) {
        return ("OK", STYLE_OK);
    }
    ("WARN", STYLE_WARN)
}

fn progress_bar(row: &RemoteStatusRow, tick: usize) -> String {
    match row {
        RemoteStatusRow::Checking { .. } => checking_bar(tick),
        RemoteStatusRow::Complete(result) if result.error.is_some() => "[!!!!!!!!!!!!]".to_string(),
        RemoteStatusRow::Complete(_) => "[============]".to_string(),
    }
}

fn checking_bar(tick: usize) -> String {
    const WIDTH: usize = 12;
    let filled = 4 + (tick % 5);
    let mut body = String::with_capacity(WIDTH);
    for index in 0..WIDTH {
        if index < filled {
            body.push('=');
        } else if index == filled {
            body.push('>');
        } else {
            body.push('.');
        }
    }
    format!("[{body}]")
}

fn optional_field(label: &str, value: Option<&str>) -> String {
    value
        .filter(|value| !value.is_empty())
        .map(|value| format!("{label}{value}"))
        .unwrap_or_default()
}

struct RowSummary {
    badge: &'static str,
    style: Style,
    detail: String,
}
