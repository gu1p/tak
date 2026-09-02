use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpListener;

mod cleanup_janitor;
mod container_ownership;
mod execution_root;
mod http_server;
#[cfg(test)]
mod http_server_request_validation_unit_tests;
#[cfg(test)]
mod http_server_test_support;
#[cfg(test)]
mod http_server_unit_tests;
mod memory_pressure_controller;
mod query_helpers;
mod resource_admission;
mod resource_baseline;
#[cfg(test)]
mod resource_baseline_tests;
mod resource_envelope;
#[cfg(test)]
mod resource_envelope_tests;
mod resource_policy;
#[cfg(test)]
mod resource_policy_tests;
mod resource_pressure_controller;
mod route_worker_v2;
mod router;
mod runtime;
mod runtime_services;
#[cfg(test)]
mod runtime_services_tests;
mod runtime_state;
#[cfg(test)]
mod runtime_tests;
mod status_resources;
mod status_state;
mod status_state_helpers;
mod submit_store;
mod tak_container_usage;
mod types;
mod worker_cache_gc;
mod worker_v2_execution;

pub use http_server::run_worker_http_server;
pub use router::handle_worker_http_request;
pub use runtime::RemoteRuntimeConfig;
pub use submit_store::{
    ActiveSubmitAttempt, SubmitAttemptStore, SubmitRegistration, build_submit_idempotency_key,
};
pub use types::{
    RemoteImageCacheRuntimeConfig, RemoteNodeContext, SubmitAttemptSummaryRecord,
    WorkerHttpResponse,
};
#[doc(hidden)]
pub use worker_v2_execution::worker_v2_cancellation_poll_requests_cancel;

pub(crate) use cleanup_janitor::spawn_remote_cleanup_janitor;
use execution_root::{artifact_root_base_for_execution_root_base, remote_execution_root_base};
pub(crate) use http_server::{handle_worker_http_stream, handle_worker_stream};
pub(crate) use memory_pressure_controller::spawn_memory_pressure_controller;
use query_helpers::{
    binary_response, error_response, protobuf_response, query_param_string, query_param_u64,
    sanitize_submit_idempotency_key, split_path_and_query, text_response,
};
use route_worker_v2::handle_worker_v2_route;
pub(crate) use runtime_services::spawn_remote_runtime_services;
pub(crate) use tak_container_usage::spawn_tak_container_usage_sampler;
use worker_v2_execution::{reserve_worker_v2_resources, spawn_worker_v2_execution};

const PROTOCOL_V2_UPGRADE_MESSAGE: &str =
    "Protocol v2 is required; upgrade tak, takd, and workers together.";
