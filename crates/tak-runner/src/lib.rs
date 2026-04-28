//! Shared step-running machinery for `tak` execution paths and `takd` remote workers.

pub use tak_exec::{
    ImageCacheOptions, OutputStream, RemoteWorkerExecutionResult, RemoteWorkerExecutionSpec,
    TaskOutputChunk, TaskOutputObserver, execute_remote_worker_steps,
    execute_remote_worker_steps_with_output, image_cache_status, run_image_cache_janitor_once,
};

extern crate self as tak_runner;
