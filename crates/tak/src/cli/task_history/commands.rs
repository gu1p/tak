use std::io::{self, Write};
use std::time::Duration;

use anyhow::Result;

use super::store::{TaskHistoryStore, TaskOutputRow};

pub(in crate::cli) fn print_task_history(limit: usize) -> Result<()> {
    println!("Local Tasks");
    let rows = match TaskHistoryStore::open_default().and_then(|store| store.list_runs(limit)) {
        Ok(rows) => rows,
        Err(err) => {
            println!(
                "history=unavailable detail={}",
                single_line(&format!("{err:#}"))
            );
            return Ok(());
        }
    };
    if rows.is_empty() {
        println!("(none)");
        return Ok(());
    }
    for row in rows {
        println!(
            "task_label={} task_run_id={} attempts={} state={} placement={} remote_node={}",
            row.task_label,
            row.task_run_id,
            row.attempts,
            row.state,
            row.placement,
            empty_none(&row.remote_node_id),
        );
    }
    Ok(())
}

pub(in crate::cli) async fn print_task_logs(
    task_run_id: &str,
    follow: bool,
    interval_ms: u64,
) -> Result<()> {
    let store = TaskHistoryStore::open_default()?;
    let mut seen = 0_usize;
    let mut polls = 0_usize;
    loop {
        let rows = store.output_rows(task_run_id)?;
        for row in rows.iter().skip(seen) {
            write_output_row(row)?;
        }
        seen = rows.len();
        if !follow {
            return Ok(());
        }
        polls = polls.saturating_add(1);
        if test_max_polls().is_some_and(|limit| polls >= limit) {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(interval_ms.max(1))).await;
    }
}

fn write_output_row(row: &TaskOutputRow) -> Result<()> {
    match row.stream.as_str() {
        "stdout" => {
            io::stdout().write_all(&row.bytes)?;
            io::stdout().flush()?;
        }
        "stderr" => {
            io::stderr().write_all(&row.bytes)?;
            io::stderr().flush()?;
        }
        _ => {}
    }
    Ok(())
}

fn empty_none(value: &str) -> &str {
    if value.is_empty() { "none" } else { value }
}

fn single_line(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn test_max_polls() -> Option<usize> {
    std::env::var("TAK_TEST_TASK_HISTORY_LOGS_MAX_POLLS")
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
}
