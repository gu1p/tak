//! Shared step-running machinery for `tak` execution paths and `takd` remote workers.

pub use tak_exec::{
    RemoteWorkerExecutionResult, RemoteWorkerExecutionSpec, execute_remote_worker_steps,
};
