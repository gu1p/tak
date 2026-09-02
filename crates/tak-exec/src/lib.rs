//! Worker-side step and container execution utilities shared by `tak-runner` and `takd`.

extern crate self as tak_exec;

mod cancellation;
mod container_engine;
mod container_runtime;
mod deadline;
mod execution_types;
mod image_cache;
mod remote_diagnostics;
mod remote_endpoint;
mod remote_worker;
mod runtime_metadata;
mod sqlite_connection;
mod step_execution;
mod step_runner;
mod worker_output;
mod worker_runtime;

pub use cancellation::{RunCancellation, RunCancelled, is_run_cancelled_error};
pub use execution_types::{
    ContainerExecutionIdentity, ImageCacheOptions, OutputStream, PlacementMode,
    RemoteWorkerExecutionOutcome, RemoteWorkerExecutionResult, RemoteWorkerExecutionSpec,
    TaskFinishedEvent, TaskOutputChunk, TaskOutputObserver, TaskStatusEvent, TaskStatusPhase,
};
pub use image_cache::{
    cached_image_content_keys, image_cache_status, run_image_cache_janitor_once,
};
pub use remote_diagnostics::{
    NoMatchingRemoteError, RemoteCandidateDiagnostic, RemoteCandidateRejection, RemoteObservation,
    RemotePreflightExhaustedError, RemotePreflightFailure, RemotePreflightFailureKind,
    RequiredRemoteDiagnostic,
};
pub use remote_worker::{
    execute_remote_worker_steps_with_cancellation,
    execute_remote_worker_steps_with_output_and_cancellation,
};
#[doc(hidden)]
pub use sqlite_connection::ProcessSqliteConnection;
pub(crate) use worker_output::emit_task_output;
pub(crate) use worker_runtime::{ContainerExecutionPlan, ImageCachePlan};

pub use remote_endpoint::{endpoint_host_port, endpoint_socket_addr, socket_addr_from_host_port};
