#![allow(dead_code)]

use std::sync::{Mutex, MutexGuard};

use tak_exec::{TaskOutputChunk, TaskOutputObserver, TaskStatusEvent};

#[derive(Default)]
pub struct CollectingStatusObserver {
    statuses: Mutex<Vec<TaskStatusEvent>>,
}

impl CollectingStatusObserver {
    pub fn snapshot(&self) -> MutexGuard<'_, Vec<TaskStatusEvent>> {
        self.statuses.lock().expect("status observer lock")
    }
}

impl TaskOutputObserver for CollectingStatusObserver {
    fn observe_output(&self, _chunk: TaskOutputChunk) -> anyhow::Result<()> {
        Ok(())
    }

    fn observe_status(&self, event: TaskStatusEvent) -> anyhow::Result<()> {
        self.statuses
            .lock()
            .expect("status observer lock")
            .push(event);
        Ok(())
    }
}
