use std::sync::Mutex;

use anyhow::Result;

use crate::engine::{TaskOutputChunk, TaskOutputObserver};

#[derive(Default)]
pub(in super::super) struct CapturingObserver {
    pub(in super::super) output: Mutex<Vec<u8>>,
}

impl TaskOutputObserver for CapturingObserver {
    fn observe_output(&self, chunk: TaskOutputChunk) -> Result<()> {
        self.output
            .lock()
            .expect("output lock")
            .extend_from_slice(&chunk.bytes);
        Ok(())
    }
}
