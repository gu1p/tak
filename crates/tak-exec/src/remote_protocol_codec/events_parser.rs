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
    for event in parsed.events {
        checkpoint = checkpoint.max(event.seq);
        if event.seq <= last_seen_seq {
            continue;
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
                event.chunk.or(event.message).unwrap_or_default().into_bytes()
            },
        });
    }
    remote_logs.sort_unstable_by_key(|chunk| chunk.seq);

    Ok(ParsedRemoteEvents {
        next_seq: checkpoint,
        done: parsed.done,
        remote_logs,
    })
}
