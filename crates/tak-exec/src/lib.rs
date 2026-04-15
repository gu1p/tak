//! Task execution engine for resolved workspace tasks.
//!
//! This crate expands target dependencies, enforces execution ordering, applies retry and
//! timeout policy, and optionally coordinates daemon leases around task execution.

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::future::Future;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use arti_client::TorClient;
use base64::Engine;
use sha2::{Digest, Sha256};
use tak_core::model::{
    ContainerRuntimeSourceSpec, CurrentStateOrigin, CurrentStateSpec, IgnoreSourceSpec, LocalSpec,
    PathAnchor, PathRef, PolicyDecisionSpec, RemoteRuntimeSpec, RemoteSpec, RemoteTransportKind,
    ResolvedTask, RetryDef, StepDef, TaskExecutionSpec, TaskLabel, WorkspaceSpec,
    build_current_state_manifest, normalize_path_ref,
};
use tak_loader::evaluate_named_policy_decision;
use tokio::net::TcpStream;
use uuid::Uuid;
use zip::write::SimpleFileOptions;

mod client_observations;
mod client_remotes;
mod client_tor;
mod container_engine;
mod container_runtime;
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
use client_remotes::configured_remote_targets;
pub use client_tor::default_client_tor_config;
use container_engine::{
    ContainerEngine, ShellContainerEngineProbe, resolve_container_engine_host_platform,
    select_container_engine_with_probe,
};
use container_runtime::run_task_steps_in_container;
use execution_graph::collect_required_labels;
use lease_client::{acquire_task_lease, release_task_lease};
use remote_endpoint::{remote_protocol_bearer_token, test_tor_onion_dial_addr};
use remote_protocol_codec::{
    build_remote_submit_payload, parse_remote_events_response, parse_remote_result_outputs,
};
use retry::{retry_backoff_delay, should_retry};
#[allow(unused_imports)]
use step_runner::resolve_cwd;
use step_runner::{StepRunResult, run_step};
pub use summary::target_set_from_summary;

include!("engine/public_types.rs");
include!("engine/output_observer.rs");
include!("engine/remote_diagnostics.rs");
include!("engine/run_tasks.rs");
include!("engine/lease_context.rs");
include!("engine/remote_models.rs");
include!("engine/preflight_failure.rs");
include!("engine/transport.rs");

fn transport_adapter_for_kind(kind: RemoteTransportKind) -> &'static dyn RemoteTransportAdapter {
    match kind {
        RemoteTransportKind::Any => {
            panic!("strict remote targets must resolve to a concrete transport")
        }
        RemoteTransportKind::Direct => &DIRECT_HTTPS_TRANSPORT_ADAPTER,
        RemoteTransportKind::Tor => &TOR_TRANSPORT_ADAPTER,
    }
}

include!("engine/attempt_submit.rs");
include!("engine/attempt_execution.rs");
include!("engine/task_result.rs");
include!("engine/run_single_task.rs");
include!("engine/workspace_stage.rs");
include!("engine/workspace_collect.rs");
include!("engine/workspace_sync.rs");
include!("engine/runtime_metadata.rs");
include!("engine/placement.rs");
include!("engine/preflight_status_output.rs");
include!("engine/preflight_fallback.rs");
include!("engine/protocol_detection.rs");
include!("engine/protocol_submit.rs");
include!("engine/remote_wait_status.rs");
include!("engine/protocol_events.rs");
include!("engine/protocol_result_http.rs");
include!("engine/step_execution.rs");
include!("engine/remote_worker.rs");

#[cfg(test)]
mod protocol_result_http_connection_cleanup_tests;
#[cfg(test)]
mod protocol_result_http_tests;
#[cfg(test)]
mod protocol_result_http_timeout_tests;

pub use remote_endpoint::{endpoint_host_port, endpoint_socket_addr, socket_addr_from_host_port};
