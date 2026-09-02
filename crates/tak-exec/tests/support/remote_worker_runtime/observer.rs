use std::sync::{Mutex, MutexGuard};

use tak_exec::{TaskOutputChunk, TaskOutputObserver};
use tokio::sync::Notify;

#[derive(Default)]
pub struct CollectingObserver {
    chunks: Mutex<Vec<TaskOutputChunk>>,
    notify: Notify,
}

impl CollectingObserver {
    pub fn snapshot(&self) -> MutexGuard<'_, Vec<TaskOutputChunk>> {
        self.chunks.lock().expect("observer lock")
    }

    pub async fn wait_for_chunks(&self, expected: usize) {
        loop {
            if self.chunks.lock().expect("observer lock").len() >= expected {
                return;
            }
            self.notify.notified().await;
        }
    }
}

impl TaskOutputObserver for CollectingObserver {
    fn observe_output(&self, chunk: TaskOutputChunk) -> anyhow::Result<()> {
        self.chunks.lock().expect("observer lock").push(chunk);
        self.notify.notify_waiters();
        Ok(())
    }
}
