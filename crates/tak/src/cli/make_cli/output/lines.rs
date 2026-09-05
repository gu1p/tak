use std::io::{self, Write};

use anyhow::Result;
use tak_core::model::TaskLabel;
use tak_exec::OutputStream;
use tak_make::ParallelOutputMode;

use super::{BufferedLine, GoalOutput, OutputState, OutputVisibility, StreamKey};

pub(super) fn complete_lines(pending: &mut Vec<u8>, bytes: &[u8]) -> Vec<Vec<u8>> {
    pending.extend_from_slice(bytes);
    let mut lines = Vec::new();
    while let Some(end) = pending.iter().position(|byte| *byte == b'\n') {
        lines.push(pending.drain(..=end).collect());
    }
    lines
}

pub(super) fn flush_partials(
    state: &mut OutputState,
    label: &TaskLabel,
    goal: &GoalOutput,
    visibility: &OutputVisibility,
) -> Result<()> {
    for stream in [OutputStream::Stdout, OutputStream::Stderr] {
        let bytes = state
            .pending
            .remove(&(label.clone(), stream_key(stream)))
            .unwrap_or_default();
        if bytes.is_empty() {
            continue;
        }
        record_make_exit_code(state, label, stream, &bytes);
        if !visibility.writes(stream) {
            continue;
        }
        match goal.mode {
            ParallelOutputMode::Live => write_prefixed(stream, &goal.name, &bytes)?,
            ParallelOutputMode::Grouped => state
                .grouped
                .entry(label.clone())
                .or_default()
                .push(BufferedLine { stream, bytes }),
        }
    }
    Ok(())
}

pub(super) fn record_make_exit_code(
    state: &mut OutputState,
    label: &TaskLabel,
    stream: OutputStream,
    line: &[u8],
) {
    if stream != OutputStream::Stderr {
        return;
    }
    if let Some(code) = make_error_code(line) {
        state.make_exit_codes.insert(label.clone(), code);
    }
}

fn make_error_code(line: &[u8]) -> Option<i32> {
    let line = String::from_utf8_lossy(line);
    ["Error code ", "Error "].into_iter().find_map(|marker| {
        let tail = line.rsplit_once(marker)?.1;
        let digits = tail
            .chars()
            .take_while(char::is_ascii_digit)
            .collect::<String>();
        digits.parse().ok()
    })
}

pub(super) fn write_prefixed(stream: OutputStream, goal: &str, line: &[u8]) -> Result<()> {
    let prefix = format!("[{goal}] ");
    match stream {
        OutputStream::Stdout => {
            let mut output = io::stdout().lock();
            output.write_all(prefix.as_bytes())?;
            output.write_all(line)?;
            output.flush()?;
        }
        OutputStream::Stderr => {
            let mut output = io::stderr().lock();
            output.write_all(prefix.as_bytes())?;
            output.write_all(line)?;
            output.flush()?;
        }
    }
    Ok(())
}

pub(super) fn stream_key(stream: OutputStream) -> StreamKey {
    match stream {
        OutputStream::Stdout => StreamKey::Stdout,
        OutputStream::Stderr => StreamKey::Stderr,
    }
}
