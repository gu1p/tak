use std::collections::BTreeMap;
use std::io::{self, Write};

use anyhow::Result;
use tak_core::model::TaskLabel;
use tak_exec::{OutputStream, TaskOutputChunk};

use super::framing::LineFramer;
use super::model::{RunState, TaskActivity};
use super::render::state_name;

const FRAGMENT_LIMIT: usize = 16 * 1024;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum StreamKey {
    Stdout,
    Stderr,
}

pub(super) struct OutputBuffers(BTreeMap<(TaskLabel, StreamKey), LineFramer>);

impl OutputBuffers {
    pub(super) fn new() -> Self {
        Self(BTreeMap::new())
    }

    pub(super) fn emit_chunk(
        &mut self,
        chunk: &TaskOutputChunk,
        root: &TaskLabel,
        placement: &str,
    ) -> Result<()> {
        let lines = self
            .0
            .entry((root.clone(), stream_key(chunk.stream)))
            .or_insert_with(|| LineFramer::new(FRAGMENT_LIMIT))
            .push(&chunk.bytes);
        for line in lines {
            write_prefixed(chunk.stream, root, placement, &line)?;
        }
        Ok(())
    }

    pub(super) fn flush_task(&mut self, root: &TaskLabel, placement: &str) -> Result<()> {
        for stream in [OutputStream::Stdout, OutputStream::Stderr] {
            if let Some(framer) = self.0.get_mut(&(root.clone(), stream_key(stream))) {
                for fragment in framer.finish() {
                    write_prefixed(stream, root, placement, &fragment)?;
                }
            }
        }
        Ok(())
    }

    pub(super) fn flush_all(&mut self, state: &RunState) -> Result<()> {
        for ((label, stream), mut framer) in std::mem::take(&mut self.0) {
            let placement = state.placement_for(&label);
            for fragment in framer.finish() {
                write_prefixed(output_stream(stream), &label, &placement, &fragment)?;
            }
        }
        Ok(())
    }
}

pub(super) fn write_status_line(
    state: &RunState,
    label: &TaskLabel,
    queue: Option<&str>,
    position: Option<usize>,
    message: &str,
) -> Result<()> {
    let root = state.display_root(label);
    let activity = state.activity_for(&root).unwrap_or(TaskActivity::Placing);
    let queue = queue.map_or_else(String::new, |queue| match position {
        Some(position) => format!(
            " {queue} #{position} ({} ahead)",
            position.saturating_sub(1)
        ),
        None => format!(" {queue} position pending"),
    });
    write_stderr(
        format!(
            "[{}] {}@{}{} — {message}\n",
            state_name(activity),
            canonical_label(&root),
            state.placement_for(&root),
            queue,
        )
        .as_bytes(),
    )
}

pub(super) fn write_stderr(bytes: &[u8]) -> Result<()> {
    let mut stderr = io::stderr().lock();
    stderr.write_all(bytes)?;
    stderr.flush()?;
    Ok(())
}

pub(super) fn write_stdout(bytes: &[u8]) -> Result<()> {
    let mut stdout = io::stdout().lock();
    stdout.write_all(bytes)?;
    stdout.flush()?;
    Ok(())
}

fn write_prefixed(
    stream: OutputStream,
    label: &TaskLabel,
    placement: &str,
    bytes: &[u8],
) -> Result<()> {
    let prefix = format!("[{}@{}] ", canonical_label(label), placement);
    match stream {
        OutputStream::Stdout => write_bytes(io::stdout().lock(), &prefix, bytes),
        OutputStream::Stderr => write_bytes(io::stderr().lock(), &prefix, bytes),
    }
}

fn write_bytes(mut writer: impl Write, prefix: &str, bytes: &[u8]) -> Result<()> {
    writer.write_all(prefix.as_bytes())?;
    writer.write_all(bytes)?;
    if !bytes.ends_with(b"\n") {
        writer.write_all(b"\n")?;
    }
    writer.flush()?;
    Ok(())
}

fn stream_key(stream: OutputStream) -> StreamKey {
    match stream {
        OutputStream::Stdout => StreamKey::Stdout,
        OutputStream::Stderr => StreamKey::Stderr,
    }
}

fn output_stream(stream: StreamKey) -> OutputStream {
    match stream {
        StreamKey::Stdout => OutputStream::Stdout,
        StreamKey::Stderr => OutputStream::Stderr,
    }
}

fn canonical_label(label: &TaskLabel) -> String {
    if label.package == "//" {
        format!("//:{}", label.name)
    } else {
        format!("{}:{}", label.package, label.name)
    }
}
