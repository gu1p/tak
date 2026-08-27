use super::{RemoteWorkerExecutionOutcome, RemoteWorkerExecutionResult};

impl RemoteWorkerExecutionOutcome {
    pub fn new(result: RemoteWorkerExecutionResult, container_oom_killed: Option<bool>) -> Self {
        Self {
            result,
            container_oom_killed,
        }
    }

    pub fn result(&self) -> &RemoteWorkerExecutionResult {
        &self.result
    }

    pub fn container_oom_killed(&self) -> Option<bool> {
        self.container_oom_killed
    }

    pub fn into_result(self) -> RemoteWorkerExecutionResult {
        self.result
    }
}

impl std::ops::Deref for RemoteWorkerExecutionOutcome {
    type Target = RemoteWorkerExecutionResult;

    fn deref(&self) -> &Self::Target {
        self.result()
    }
}
