use std::io::{self, Write};
use std::sync::Mutex;

use anyhow::{Error, Result, anyhow};
use tak_core::model::TaskLabel;
use tak_exec::{
    TaskFinishedEvent, TaskOutputChunk, TaskOutputObserver, TaskStartedEvent, TaskStatusEvent,
};

use super::store::TaskHistoryWriter;
use crate::cli::run_output::StdStreamOutputObserver;

pub(in crate::cli) struct HistoryOutputObserver {
    inner: Option<StdStreamOutputObserver>,
    history: Mutex<HistoryRecorder>,
}

struct HistoryRecorder {
    writer: Option<TaskHistoryWriter>,
    disabled_reason: Option<String>,
    warned: bool,
}

impl HistoryOutputObserver {
    pub(in crate::cli) fn new() -> Self {
        Self {
            inner: Some(StdStreamOutputObserver::default()),
            history: Mutex::new(HistoryRecorder::open()),
        }
    }

    pub(in crate::cli) fn new_recording_only() -> Self {
        Self {
            inner: None,
            history: Mutex::new(HistoryRecorder::open()),
        }
    }

    fn record_history(
        &self,
        action: impl FnOnce(&mut TaskHistoryWriter) -> Result<()>,
    ) -> Result<()> {
        let mut history = self
            .history
            .lock()
            .map_err(|_| anyhow!("task history lock poisoned"))?;
        history.record(action)
    }
}

impl TaskOutputObserver for HistoryOutputObserver {
    fn observe_output(&self, chunk: TaskOutputChunk) -> Result<()> {
        if let Some(inner) = &self.inner {
            inner.observe_output(chunk.clone())?;
        }
        self.record_history(|writer| {
            writer.record_started(
                &chunk.task_run_id,
                &canonical_task_label(&chunk.task_label),
                chunk.attempt,
            )?;
            writer.append_output(&chunk)
        })
    }

    fn observe_status(&self, event: TaskStatusEvent) -> Result<()> {
        if let Some(inner) = &self.inner {
            inner.observe_status(event)?;
        }
        Ok(())
    }

    fn observe_task_started(&self, event: TaskStartedEvent) -> Result<()> {
        self.record_history(|writer| {
            writer.record_started_event(&event, &canonical_task_label(&event.task_label))
        })
    }

    fn observe_task_finished(&self, event: TaskFinishedEvent) -> Result<()> {
        self.record_history(|writer| writer.record_finished(&event))
    }
}

impl HistoryRecorder {
    fn open() -> Self {
        match TaskHistoryWriter::open_default() {
            Ok(writer) => Self {
                writer: Some(writer),
                disabled_reason: None,
                warned: false,
            },
            Err(err) => Self::disabled(err),
        }
    }

    fn disabled(error: Error) -> Self {
        Self {
            writer: None,
            disabled_reason: Some(format!("{error:#}")),
            warned: false,
        }
    }

    fn record(&mut self, action: impl FnOnce(&mut TaskHistoryWriter) -> Result<()>) -> Result<()> {
        let Some(writer) = self.writer.as_mut() else {
            return self.warn_once();
        };

        if let Err(err) = action(writer) {
            self.writer = None;
            self.disabled_reason = Some(format!("{err:#}"));
            return self.warn_once();
        }
        Ok(())
    }

    fn warn_once(&mut self) -> Result<()> {
        if self.warned {
            return Ok(());
        }
        self.warned = true;
        let detail = self.disabled_reason.as_deref().unwrap_or("unknown error");
        let mut stderr = io::stderr().lock();
        writeln!(
            stderr,
            "warning: local task history unavailable: {}",
            single_line(detail)
        )?;
        stderr.flush()?;
        Ok(())
    }
}

fn canonical_task_label(label: &TaskLabel) -> String {
    if label.package == "//" {
        format!("//:{}", label.name)
    } else {
        format!("{}:{}", label.package, label.name)
    }
}

fn single_line(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}
