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
        if event.seq <= last_seen_seq || event.kind != "TASK_LOG_CHUNK" {
            continue;
        }
        remote_logs.push(RemoteLogChunk {
            seq: event.seq,
            chunk: event.chunk.or(event.message).unwrap_or_default(),
        });
    }
    remote_logs.sort_unstable_by_key(|chunk| chunk.seq);

    Ok(ParsedRemoteEvents {
        next_seq: checkpoint,
        done: parsed.done,
        remote_logs,
    })
}
