//! Task execution engine for resolved workspace tasks.
//!
//! This crate expands target dependencies, enforces execution ordering, applies retry and
//! timeout policy, and optionally coordinates daemon leases around task execution.

extern crate self as tak_exec;

mod client_observations;
mod client_remotes;
mod client_tor;
mod container_engine;
mod container_runtime;
mod engine;
mod execution_graph;
mod lease_client;
mod remote_endpoint;
mod remote_protocol_codec;
mod retry;
mod step_runner;
mod summary;

pub use client_observations::{
    RemoteObservation, load_remote_observation, load_remote_observation_at,
    record_remote_observation, write_remote_observation, write_remote_observation_at,
};
pub use client_tor::default_client_tor_config;
pub(crate) use engine::{
    ContainerExecutionPlan, LeaseContext, ParsedRemoteEvents, RemoteTargetSelection,
    RemoteWorkspaceStage, StrictRemoteTarget, emit_task_output,
};
pub use engine::{
    NoMatchingRemoteError, OutputStream, PlacementMode, RemoteCandidateDiagnostic,
    RemoteCandidateRejection, RemoteLogChunk, RemotePreflightExhaustedError,
    RemotePreflightFailure, RemotePreflightFailureKind, RemoteWorkerExecutionResult,
    RemoteWorkerExecutionSpec, RequiredRemoteDiagnostic, RunOptions, RunSummary, SyncedOutput,
    TaskOutputChunk, TaskOutputObserver, TaskRunResult, TaskStatusEvent, TaskStatusPhase,
    execute_remote_worker_steps, execute_remote_worker_steps_with_output, run_resolved_task,
    run_tasks,
};
pub use summary::target_set_from_summary;
#[path = "client_remotes_tests.rs"]
mod client_remotes_tests;
#[path = "engine/preflight_failure_classification_tests.rs"]
mod preflight_failure_classification_tests;
#[path = "engine/preflight_fallback_classification_tests.rs"]
mod preflight_fallback_classification_tests;
mod protocol_result_http_connection_cleanup_tests;
mod protocol_result_http_tests;
mod protocol_result_http_timeout_tests;

pub use remote_endpoint::{endpoint_host_port, endpoint_socket_addr, socket_addr_from_host_port};
