use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow, bail};
use base64::Engine;
use futures::StreamExt;
use safelog::DisplayRedacted;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tak_core::model::{RemoteRuntimeSpec, StepDef, normalize_path_ref};
use tak_exec::{RemoteWorkerExecutionSpec, execute_remote_worker_steps};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tor_cell::relaycell::msg::Connected;
use zip::read::ZipArchive;

use crate::daemon::transport::TorHiddenServiceRuntimeConfig;

mod http_server;
mod query_helpers;
mod route_events;
mod route_node;
mod route_outputs;
mod route_result;
mod route_submit;
mod router;
mod submit_payload_parse;
mod submit_store;
mod tor_server;
mod types;
mod worker_submit_execution;
mod worker_workspace_outputs;

pub use http_server::run_remote_v1_http_server;
pub use router::handle_remote_v1_request;
pub use submit_store::{SubmitAttemptStore, SubmitRegistration, build_submit_idempotency_key};
pub use tor_server::run_remote_v1_tor_hidden_service;
pub use types::RemoteV1Response;

use http_server::handle_remote_v1_http_stream;
use query_helpers::{
    execution_root_for_submit_key, json_response, query_param_string, query_param_u64,
    remote_task_path_arg, resolve_submit_idempotency_key_for_task_run, split_path_and_query,
    unix_epoch_ms,
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
use worker_submit_execution::spawn_remote_worker_submit_execution;
use worker_workspace_outputs::{
    changed_remote_worker_outputs, snapshot_workspace_files, unpack_remote_worker_workspace,
};
