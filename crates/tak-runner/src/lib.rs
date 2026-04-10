//! Shared step-running machinery for `tak` execution paths and `takd` remote workers.

pub use tak_exec::{
    OutputStream, RemoteWorkerExecutionResult, RemoteWorkerExecutionSpec, TaskOutputChunk,
    TaskOutputObserver, execute_remote_worker_steps, execute_remote_worker_steps_with_output,
};
