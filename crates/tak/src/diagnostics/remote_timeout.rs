use anyhow::Error;
use ratatui::text::{Line, Span};

use super::{STYLE_ERROR, STYLE_SECTION};

pub(super) fn remote_node_info_timeout_lines(err: &Error) -> Option<Vec<Line<'static>>> {
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
            Span::raw(
                rendered
                    .lines()
                    .next()
                    .unwrap_or("unknown error")
                    .to_string(),
            ),
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
