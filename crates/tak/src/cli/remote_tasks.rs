use anyhow::{Context, Result, bail};
use prost::Message;
use std::io::{self, Write};
use std::time::Duration;
use tak_proto::{ListTaskAttemptsResponse, PollTaskEventsResponse, RemoteEvent};

use super::remote_http::get_remote_bytes;
use super::remote_logs::selected_remote;

pub(super) async fn run_remote_tasks(node_id: &str, active: bool, limit: usize) -> Result<()> {
    let remote = selected_remote(node_id)?;
    let state = if active { "active" } else { "all" };
    let path = format!("/v1/tasks?state={state}&limit={limit}");
    let (status, body) = get_remote_bytes(&remote, &path).await?;
    if status != 200 {
        bail!(
            "remote node {} task list failed with HTTP {status}",
            remote.node_id
        );
    }
    let tasks = ListTaskAttemptsResponse::decode(body.as_slice())
        .context("decode remote task list protobuf")?;
    print!("{}", render_remote_tasks(&remote.node_id, &tasks));
    Ok(())
}

pub(super) async fn run_remote_task_logs(
    node_id: &str,
    task_run_id: &str,
    attempt: Option<u32>,
    follow: bool,
    interval_ms: u64,
) -> Result<()> {
    let remote = selected_remote(node_id)?;
    let mut last_seen_seq = 0_u64;
    let mut polls = 0_usize;
    loop {
        let path = task_events_path(task_run_id, last_seen_seq, attempt);
        let (status, body) = get_remote_bytes(&remote, &path).await?;
        if status != 200 {
            bail!(
                "remote node {} task logs failed with HTTP {status}",
                remote.node_id
            );
        }
        let events = PollTaskEventsResponse::decode(body.as_slice())
            .context("decode remote task events protobuf")?;
        for event in &events.events {
            last_seen_seq = last_seen_seq.max(event.seq);
            write_log_event(event)?;
        }
        if !follow || events.done {
            return Ok(());
        }
        polls = polls.saturating_add(1);
        if test_max_polls().is_some_and(|limit| polls >= limit) {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(interval_ms.max(1))).await;
    }
}

fn render_remote_tasks(node_id: &str, tasks: &ListTaskAttemptsResponse) -> String {
    let mut output = String::from("Remote Tasks\n");
    if tasks.attempts.is_empty() {
        output.push_str("(none)\n");
        return output;
    }
    for task in &tasks.attempts {
        output.push_str(&format!(
            "node={} task_label={} task_run_id={} attempt={} state={}\n",
            if task.node_id.is_empty() {
                node_id
            } else {
                task.node_id.as_str()
            },
            empty_unknown(display_task_label(
                &task.task_label,
                task.execution_label.as_deref()
            )),
            task.task_run_id,
            task.attempt,
            task.state,
        ));
    }
    output
}

fn task_events_path(task_run_id: &str, after_seq: u64, attempt: Option<u32>) -> String {
    let encoded_task_run_id: String =
        url::form_urlencoded::byte_serialize(task_run_id.as_bytes()).collect();
    let mut path = format!("/v1/tasks/{encoded_task_run_id}/events?after_seq={after_seq}");
    if let Some(attempt) = attempt {
        path.push_str(&format!("&attempt={attempt}"));
    }
    path
}

fn write_log_event(event: &RemoteEvent) -> Result<()> {
    match event.kind.as_str() {
        "TASK_STDOUT_CHUNK" | "TASK_LOG_CHUNK" => write_chunk(&mut io::stdout(), event)?,
        "TASK_STDERR_CHUNK" => write_chunk(&mut io::stderr(), event)?,
        "TASK_FAILED" | "TASK_CANCELLED" | "TASK_TERMINAL" => {
            write_terminal_message(&mut io::stderr(), event)?
        }
        _ => {}
    }
    Ok(())
}

fn write_terminal_message(writer: &mut impl Write, event: &RemoteEvent) -> Result<()> {
    let Some(message) = terminal_message(event) else {
        return Ok(());
    };
    writer.write_all(message.as_bytes())?;
    if !message.ends_with('\n') {
        writer.write_all(b"\n")?;
    }
    writer.flush()?;
    Ok(())
}

fn terminal_message(event: &RemoteEvent) -> Option<String> {
    if let Some(message) = event
        .message
        .as_deref()
        .filter(|message| !message.is_empty())
    {
        return Some(message.to_string());
    }
    let failure_verb = match event.kind.as_str() {
        "TASK_CANCELLED" => "cancelled",
        "TASK_FAILED" | "TASK_TERMINAL" => "failed",
        _ => return None,
    };
    (event.success == Some(false))
        .then_some(event.exit_code)
        .flatten()
        .map(|exit_code| format!("remote task {failure_verb} with exit code {exit_code}"))
}

fn write_chunk(writer: &mut impl Write, event: &RemoteEvent) -> Result<()> {
    writer.write_all(&event.chunk_bytes)?;
    writer.flush()?;
    Ok(())
}

fn empty_unknown(value: &str) -> &str {
    if value.trim().is_empty() {
        "(unknown)"
    } else {
        value
    }
}

fn display_task_label<'a>(task_label: &'a str, execution_label: Option<&'a str>) -> &'a str {
    execution_label
        .filter(|label| !label.trim().is_empty())
        .unwrap_or(task_label)
}

fn test_max_polls() -> Option<usize> {
    std::env::var("TAK_TEST_REMOTE_TASK_LOGS_MAX_POLLS")
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
}
