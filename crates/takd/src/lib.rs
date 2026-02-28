//! Tak daemon protocol and lease coordination engine.
//!
//! The daemon serves NDJSON requests over a Unix socket and coordinates machine-wide
//! limiter leases with optional SQLite-backed persistence and history.

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow, bail};
use base64::Engine;
use futures::StreamExt;
use rusqlite::{Connection, ErrorCode, params};
use safelog::DisplayRedacted;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tak_core::label::parse_label;
use tak_core::model::{RemoteRuntimeSpec, Scope, StepDef, normalize_path_ref};
use tak_exec::{RemoteWorkerExecutionSpec, RunOptions, execute_remote_worker_steps, run_tasks};
use tak_loader::{LoadOptions, load_workspace};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream, UnixListener, UnixStream};
use tor_cell::relaycell::msg::Connected;
use uuid::Uuid;
use zip::read::ZipArchive;

#[allow(dead_code)]
const _TOR_HIDDEN_SERVICE_CONTRACT_MARKER: &str = "arti_client::TorClient launch_onion_service(";

include!("daemon/protocol_types.rs");
include!("daemon/container_engine_types.rs");
include!("daemon/tor_transport_config.rs");
include!("daemon/container_engine_selection.rs");
include!("daemon/lease_models.rs");
include!("daemon/lease_manager_public_methods.rs");
include!("daemon/lease_manager_allocation_methods.rs");
include!("daemon/lease_manager_persistence_load.rs");
include!("daemon/lease_manager_persistence_store.rs");
include!("daemon/lease_manager_shared.rs");
include!("daemon/unix_server.rs");
include!("daemon/remote_http_server.rs");
include!("daemon/remote_tor_server.rs");
include!("daemon/local_protocol_io.rs");
include!("daemon/daemon_dispatch.rs");
include!("daemon/run_tasks_request.rs");
include!("daemon/daemon_runtime.rs");
include!("daemon/default_paths_and_validation.rs");
include!("daemon/remote_submit_types.rs");
include!("daemon/submit_store_core_methods.rs");
include!("daemon/submit_store_query_methods.rs");
include!("daemon/submit_store_helpers.rs");
include!("daemon/remote_v1_router.rs");
include!("daemon/remote_v1_route_node.rs");
include!("daemon/remote_v1_route_submit.rs");
include!("daemon/remote_v1_route_events.rs");
include!("daemon/remote_v1_route_outputs.rs");
include!("daemon/remote_v1_route_result.rs");
include!("daemon/remote_submit_payload_parse.rs");
include!("daemon/remote_worker_submit_execution.rs");
include!("daemon/remote_worker_workspace_outputs.rs");
include!("daemon/remote_v1_query_helpers.rs");
