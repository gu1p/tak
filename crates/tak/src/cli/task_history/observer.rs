use std::sync::Mutex;

use anyhow::{Result, anyhow};
use tak_core::model::TaskLabel;
use tak_exec::{
    TaskFinishedEvent, TaskOutputChunk, TaskOutputObserver, TaskStartedEvent, TaskStatusEvent,
};

use super::store::TaskHistoryStore;
use crate::cli::run_output::StdStreamOutputObserver;

pub(in crate::cli) struct HistoryOutputObserver {
    inner: StdStreamOutputObserver,
    store: TaskHistoryStore,
    history_lock: Mutex<()>,
}

impl HistoryOutputObserver {
    pub(in crate::cli) fn new(store: TaskHistoryStore) -> Self {
        Self {
            inner: StdStreamOutputObserver::default(),
            store,
            history_lock: Mutex::new(()),
        }
    }
}

impl TaskOutputObserver for HistoryOutputObserver {
    fn observe_output(&self, chunk: TaskOutputChunk) -> Result<()> {
        self.inner.observe_output(chunk.clone())?;
        let _guard = self
            .history_lock
            .lock()
            .map_err(|_| anyhow!("task history lock poisoned"))?;
        self.store.record_started(
            &chunk.task_run_id,
            &canonical_task_label(&chunk.task_label),
            chunk.attempt,
        )?;
        self.store.append_output(&chunk)
    }

    fn observe_status(&self, event: TaskStatusEvent) -> Result<()> {
        self.inner.observe_status(event)
    }

    fn observe_task_started(&self, event: TaskStartedEvent) -> Result<()> {
        let _guard = self
            .history_lock
            .lock()
            .map_err(|_| anyhow!("task history lock poisoned"))?;
        self.store
            .record_started_event(&event, &canonical_task_label(&event.task_label))
    }

    fn observe_task_finished(&self, event: TaskFinishedEvent) -> Result<()> {
        let _guard = self
            .history_lock
            .lock()
            .map_err(|_| anyhow!("task history lock poisoned"))?;
        self.store.record_finished(&event)
    }
}

fn canonical_task_label(label: &TaskLabel) -> String {
    if label.package == "//" {
        format!("//:{}", label.name)
    } else {
        format!("{}:{}", label.package, label.name)
    }
}
