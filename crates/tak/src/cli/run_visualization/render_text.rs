use super::model::{TaskActivity, TaskRow};
use super::render_style::state_name;

pub(super) fn fit(value: &str, width: usize) -> String {
    if value.chars().count() <= width {
        return value.to_string();
    }
    value
        .chars()
        .take(width.saturating_sub(1))
        .chain(std::iter::once('…'))
        .collect()
}

pub(super) fn task_name(row: &TaskRow) -> String {
    let label = canonical_label(&row.label);
    match row.member_count.saturating_sub(1) {
        0 => label,
        1 => format!("{label} (+1 task)"),
        extra => format!("{label} (+{extra} tasks)"),
    }
}

pub(super) fn placement(row: &TaskRow) -> String {
    let node = placement_node(row);
    match row.transport.as_deref() {
        Some(transport) if node != "local" && node != "pending" => format!("{node}/{transport}"),
        _ => node,
    }
}

pub(super) fn placement_node(row: &TaskRow) -> String {
    row.node.clone().unwrap_or_else(|| "pending".into())
}

pub(super) fn activity_text(row: &TaskRow) -> String {
    let state = state_name(row.activity);
    let Some(queue) = row.queue_id.as_deref() else {
        return state.into();
    };
    match row.queue_position {
        Some(position) => format!(
            "{state} · {queue} #{position} ({} ahead)",
            position.saturating_sub(1)
        ),
        None => format!("{state} · {queue} position pending"),
    }
}

pub(super) fn elapsed(row: &TaskRow) -> String {
    let duration = row
        .finished_elapsed
        .unwrap_or_else(|| row.started_at.elapsed());
    if duration.as_secs() == 0 {
        "<1s".into()
    } else if duration.as_secs() < 60 {
        format!("{}s", duration.as_secs())
    } else {
        format!(
            "{}m {:02}s",
            duration.as_secs() / 60,
            duration.as_secs() % 60
        )
    }
}

pub(super) fn footer(rows: &[&TaskRow]) -> String {
    [
        TaskActivity::Passed,
        TaskActivity::Failed,
        TaskActivity::Cancelled,
        TaskActivity::Running,
        TaskActivity::Uploading,
        TaskActivity::Staging,
        TaskActivity::Queued,
        TaskActivity::Placing,
        TaskActivity::Retrying,
        TaskActivity::Syncing,
        TaskActivity::Waiting,
    ]
    .into_iter()
    .filter_map(|activity| {
        let count = rows.iter().filter(|row| row.activity == activity).count();
        (count > 0).then(|| format!("{count} {}", state_name(activity)))
    })
    .collect::<Vec<_>>()
    .join(" · ")
}

fn canonical_label(label: &tak_core::model::TaskLabel) -> String {
    if label.package == "//" {
        format!("//:{}", label.name)
    } else {
        format!("{}:{}", label.package, label.name)
    }
}
