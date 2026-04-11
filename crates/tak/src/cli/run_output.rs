use std::io::{self, Write};
use std::sync::Mutex;

use tak_exec::{TaskOutputChunk, TaskOutputObserver};

use super::*;

#[derive(Default)]
pub(super) struct StdStreamOutputObserver {
    stdout_lock: Mutex<()>,
    stderr_lock: Mutex<()>,
}

impl TaskOutputObserver for StdStreamOutputObserver {
    fn observe_output(&self, chunk: TaskOutputChunk) -> Result<()> {
        match chunk.stream {
            tak_exec::OutputStream::Stdout => {
                let _guard = self
                    .stdout_lock
                    .lock()
                    .map_err(|_| anyhow!("stdout observer lock poisoned"))?;
                let mut stdout = io::stdout().lock();
                stdout.write_all(&chunk.bytes)?;
                stdout.flush()?;
            }
            tak_exec::OutputStream::Stderr => {
                let _guard = self
                    .stderr_lock
                    .lock()
                    .map_err(|_| anyhow!("stderr observer lock poisoned"))?;
                let mut stderr = io::stderr().lock();
                stderr.write_all(&chunk.bytes)?;
                stderr.flush()?;
            }
        }
        Ok(())
    }
}
