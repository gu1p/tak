use std::sync::Arc;

use anyhow::Result;
use tak_core::model::TaskLabel;

use super::remote_models::RemoteProtocolResult;
use super::{OutputStream, RemoteLogChunk, TaskOutputObserver, emit_task_output};

pub(crate) fn recover_missing_remote_result_tails(
    task_run_id: &str,
    task_label: &TaskLabel,
    attempt: u32,
    output_observer: Option<&Arc<dyn TaskOutputObserver>>,
    remote_logs: &mut Vec<RemoteLogChunk>,
    result: &RemoteProtocolResult,
) -> Result<()> {
    let mut next_seq = next_synthetic_seq(remote_logs);
    next_seq = recover_missing_stream_tail(
        RemoteResultTail {
            stream: OutputStream::Stdout,
            value: result.stdout_tail.as_deref(),
        },
        task_run_id,
        task_label,
        attempt,
        output_observer,
        remote_logs,
        next_seq,
    )?;
    recover_missing_stream_tail(
        RemoteResultTail {
            stream: OutputStream::Stderr,
            value: result.stderr_tail.as_deref(),
        },
        task_run_id,
        task_label,
        attempt,
        output_observer,
        remote_logs,
        next_seq,
    )?;
    Ok(())
}

struct RemoteResultTail<'a> {
    stream: OutputStream,
    value: Option<&'a str>,
}

fn recover_missing_stream_tail(
    tail: RemoteResultTail<'_>,
    task_run_id: &str,
    task_label: &TaskLabel,
    attempt: u32,
    output_observer: Option<&Arc<dyn TaskOutputObserver>>,
    remote_logs: &mut Vec<RemoteLogChunk>,
    seq: u64,
) -> Result<u64> {
    let Some(value) = tail.value.filter(|value| !value.is_empty()) else {
        return Ok(seq);
    };
    if has_stream(remote_logs, tail.stream) {
        return Ok(seq);
    }
    let bytes = value.as_bytes().to_vec();
    emit_task_output(
        output_observer,
        task_run_id,
        task_label,
        attempt,
        tail.stream,
        &bytes,
    )?;
    remote_logs.push(RemoteLogChunk {
        seq,
        stream: tail.stream,
        bytes,
    });
    Ok(seq.saturating_add(1))
}

fn has_stream(remote_logs: &[RemoteLogChunk], stream: OutputStream) -> bool {
    remote_logs.iter().any(|chunk| chunk.stream == stream)
}

fn next_synthetic_seq(remote_logs: &[RemoteLogChunk]) -> u64 {
    remote_logs
        .iter()
        .map(|chunk| chunk.seq)
        .max()
        .unwrap_or(0)
        .saturating_add(1)
}
