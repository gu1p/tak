use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use anyhow::{Result, anyhow};

#[derive(Clone, Default)]
pub(super) struct DaemonTaskHandles {
    inner: Arc<Mutex<BTreeMap<String, DaemonTaskHandle>>>,
}

#[derive(Debug, Clone)]
pub(super) struct DaemonTaskHandle {
    pub(super) node_id: String,
    pub(super) task_run_id: String,
}

impl DaemonTaskHandles {
    pub(super) fn register(&self, node_id: &str, task_run_id: &str) -> Result<String> {
        let handle = format!("remote:{node_id}:{task_run_id}");
        let task = DaemonTaskHandle {
            node_id: node_id.to_string(),
            task_run_id: task_run_id.to_string(),
        };
        self.inner
            .lock()
            .map_err(|_| anyhow!("daemon task registry lock poisoned"))?
            .insert(handle.clone(), task);
        Ok(handle)
    }

    pub(super) fn resolve(&self, handle: &str) -> Result<DaemonTaskHandle> {
        self.inner
            .lock()
            .map_err(|_| anyhow!("daemon task registry lock poisoned"))?
            .get(handle)
            .cloned()
            .ok_or_else(|| {
                anyhow!("daemon task handle not found; local takd may have restarted: {handle}")
            })
    }
}
