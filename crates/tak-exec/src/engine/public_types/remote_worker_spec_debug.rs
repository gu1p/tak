use std::collections::BTreeMap;

use super::RemoteWorkerExecutionSpec;

impl std::fmt::Debug for RemoteWorkerExecutionSpec {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let environment = self
            .base_environment
            .keys()
            .map(|name| (name.as_str(), "<redacted>"))
            .collect::<BTreeMap<_, _>>();
        formatter
            .debug_struct("RemoteWorkerExecutionSpec")
            .field("task_label", &self.task_label)
            .field("task_run_id", &self.task_run_id)
            .field("attempt", &self.attempt)
            .field("step_count", &self.steps.len())
            .field("base_environment", &environment)
            .field("clear_environment", &self.clear_environment)
            .field("timeout_s", &self.timeout_s)
            .field("runtime", &self.runtime)
            .field("node_id", &self.node_id)
            .field("container_user", &self.container_user)
            .field("image_cache", &self.image_cache)
            .field("container_identity", &self.container_identity)
            .finish()
    }
}
