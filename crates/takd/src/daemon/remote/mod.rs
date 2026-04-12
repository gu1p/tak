use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow, bail};
use futures::StreamExt;
use safelog::DisplayRedacted;
use sha2::{Digest, Sha256};
use tak_core::label::parse_label;
use tak_core::model::{RemoteRuntimeSpec, StepDef, normalize_path_ref};
use tak_runner::{
    OutputStream, RemoteWorkerExecutionSpec, TaskOutputChunk, TaskOutputObserver,
    execute_remote_worker_steps_with_output,
};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tor_cell::relaycell::msg::Connected;
use zip::read::ZipArchive;

use crate::daemon::transport::TorHiddenServiceRuntimeConfig;

mod cleanup_janitor;
mod http_server;
mod query_helpers;
mod route_events;
mod route_node;
mod route_outputs;
mod route_result;
mod route_submit;
mod router;
mod status_state;
mod status_state_helpers;
mod submit_payload_parse;
mod submit_store;
mod tor_server;
mod types;
mod worker_output_artifacts;
mod worker_submit_execution;
mod worker_workspace_outputs;

pub use http_server::run_remote_v1_http_server;
pub use router::handle_remote_v1_request;
pub use submit_store::{SubmitAttemptStore, SubmitRegistration, build_submit_idempotency_key};
pub use tor_server::run_remote_v1_tor_hidden_service;
pub use types::{RemoteNodeContext, RemoteV1Response};

pub(crate) use cleanup_janitor::spawn_remote_cleanup_janitor;
pub(crate) use http_server::handle_remote_v1_http_stream;
use query_helpers::{
    artifact_root_for_submit_key, binary_response, error_response, execution_root_for_submit_key,
    protobuf_response, query_param_string, query_param_u64, remote_artifact_root_base,
    remote_execution_root_base, remote_task_path_arg, resolve_submit_idempotency_key_for_task_run,
    sanitize_submit_idempotency_key, split_path_and_query, unix_epoch_ms,
};
use route_events::handle_remote_events_route;
use route_node::{handle_node_metadata_route, handle_remote_cancel_route};
use route_outputs::handle_remote_outputs_route;
use route_result::handle_remote_result_route;
use route_submit::handle_remote_submit_route;
use submit_payload_parse::parse_remote_worker_submit_payload;
pub(crate) use tor_server::{
    remote_v1_bind_addr_from_env, tor_hidden_service_runtime_config_from_env,
};
use types::{RemoteWorkerOutputRecord, RemoteWorkerSubmitPayload, WorkspaceFileFingerprint};
use worker_output_artifacts::{read_staged_remote_output, stage_remote_worker_outputs};
use worker_submit_execution::spawn_remote_worker_submit_execution;
use worker_workspace_outputs::{
    changed_remote_worker_outputs, snapshot_workspace_files, unpack_remote_worker_workspace,
};

pub(crate) fn remote_node_context_from_env(base_url: Option<String>) -> RemoteNodeContext {
    RemoteNodeContext::new(
        tak_proto::NodeInfo {
            node_id: env_or("TAKD_NODE_ID", "local"),
            display_name: env_or("TAKD_DISPLAY_NAME", "local"),
            base_url: base_url
                .unwrap_or_else(|| env_or("TAKD_ADVERTISE_URL", "http://127.0.0.1:0")),
            healthy: true,
            pools: env_list("TAKD_NODE_POOLS", "default"),
            tags: env_list("TAKD_NODE_TAGS", "builder"),
            capabilities: env_list("TAKD_NODE_CAPABILITIES", "linux"),
            transport: env_or("TAKD_NODE_TRANSPORT", "direct"),
        },
        std::env::var("TAKD_BEARER_TOKEN").unwrap_or_default(),
    )
}

fn env_or(name: &str, default: &str) -> String {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| default.to_string())
}

fn env_list(name: &str, default: &str) -> Vec<String> {
    env_or(name, default)
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}
