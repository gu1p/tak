use super::*;

pub(crate) fn parse_remote_events_response(
    target: &StrictRemoteTarget,
    response_body: &[u8],
    last_seen_seq: u64,
) -> Result<ParsedRemoteEvents> {
    let parsed = PollTaskEventsResponse::decode(response_body).with_context(|| {
        format!(
            "infra error: remote node {} returned invalid protobuf for events",
            target.node_id
        )
    })?;

    let mut checkpoint = last_seen_seq;
    let mut remote_logs = Vec::new();
    let mut status_messages = Vec::new();
    let mut status_updates = Vec::new();
    for event in parsed.events {
        checkpoint = checkpoint.max(event.seq);
        if event.seq <= last_seen_seq {
            continue;
        }
        if let Some(update) = event_status_update(&event) {
            status_messages.push(update.message.clone());
            status_updates.push(update);
        }
        let stream = match event.kind.as_str() {
            "TASK_STDOUT_CHUNK" | "TASK_LOG_CHUNK" => Some(OutputStream::Stdout),
            "TASK_STDERR_CHUNK" => Some(OutputStream::Stderr),
            _ => None,
        };
        let Some(stream) = stream else {
            continue;
        };
        remote_logs.push(RemoteLogChunk {
            seq: event.seq,
            stream,
            bytes: if !event.chunk_bytes.is_empty() {
                event.chunk_bytes
            } else {
                event
                    .chunk
                    .or(event.message)
                    .unwrap_or_default()
                    .into_bytes()
            },
        });
    }
    remote_logs.sort_unstable_by_key(|chunk| chunk.seq);

    Ok(ParsedRemoteEvents {
        next_seq: checkpoint,
        done: parsed.done,
        remote_logs,
        status_messages,
        status_updates,
    })
}

fn event_status_update(event: &tak_proto::RemoteEvent) -> Option<RemoteStatusUpdate> {
    if matches!(
        event.kind.as_str(),
        "TASK_QUEUED" | "TASK_QUEUE_POSITION" | "TASK_FAILED" | "TASK_CANCELLED" | "TASK_TERMINAL"
    ) && let Some(message) = event
        .message
        .as_deref()
        .filter(|message| !message.is_empty())
    {
        return Some(RemoteStatusUpdate {
            message: message.to_string(),
            kind: event_status_kind(event.kind.as_str()),
            queue_position: queue_position_from_message(message),
        });
    }
    let failure_verb = terminal_failure_verb(event.kind.as_str())?;
    if event.success == Some(false)
        && let Some(exit_code) = event.exit_code
    {
        return Some(RemoteStatusUpdate {
            message: format!("remote task {failure_verb} with exit code {exit_code}"),
            kind: event_status_kind(event.kind.as_str()),
            queue_position: None,
        });
    }
    None
}

fn event_status_kind(kind: &str) -> TaskStatusEventKind {
    match kind {
        "TASK_QUEUED" => TaskStatusEventKind::QueueAdmission,
        "TASK_QUEUE_POSITION" => TaskStatusEventKind::QueuePositionChanged,
        "TASK_CANCELLED" => TaskStatusEventKind::Cancellation,
        "TASK_FAILED" | "TASK_TERMINAL" => TaskStatusEventKind::FatalFailure,
        _ => TaskStatusEventKind::RemoteExecutionStart,
    }
}

fn queue_position_from_message(message: &str) -> Option<usize> {
    let (_, tail) = message.split_once("queue position: ")?;
    let value = tail
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect::<String>();
    value.parse().ok()
}

fn terminal_failure_verb(kind: &str) -> Option<&'static str> {
    match kind {
        "TASK_CANCELLED" => Some("cancelled"),
        "TASK_FAILED" | "TASK_TERMINAL" => Some("failed"),
        _ => None,
    }
}
