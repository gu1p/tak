//! Shared step-running machinery for `tak` execution paths and `takd` remote workers.

pub use tak_exec::{
    ContainerExecutionIdentity, ImageCacheOptions, OutputStream, RemoteWorkerExecutionResult,
    RemoteWorkerExecutionSpec, RunCancellation, TaskOutputChunk, TaskOutputObserver,
    execute_remote_worker_steps, execute_remote_worker_steps_with_cancellation,
    execute_remote_worker_steps_with_output,
    execute_remote_worker_steps_with_output_and_cancellation, image_cache_status,
    is_run_cancelled_error, run_image_cache_janitor_once,
};

extern crate self as tak_runner;
