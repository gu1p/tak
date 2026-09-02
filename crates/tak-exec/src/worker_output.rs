use anyhow::Result;
use tak_core::model::TaskLabel;

use crate::{OutputStream, TaskOutputChunk, TaskOutputObserver};

pub(crate) fn emit_task_output(
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
    task_run_id: &str,
    task_label: &TaskLabel,
    attempt: u32,
    stream: OutputStream,
    bytes: &[u8],
) -> Result<()> {
    if bytes.is_empty() {
        return Ok(());
    }
    let Some(observer) = output_observer else {
        return Ok(());
    };
    observer.observe_output(TaskOutputChunk {
        task_run_id: task_run_id.to_string(),
        task_label: task_label.clone(),
        attempt,
        stream,
        bytes: bytes.to_vec(),
    })
}
