use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use tak_core::model::Scope;
use tokio::net::TcpListener;

use crate::daemon::lease::new_shared_manager_with_db;
use crate::daemon::protocol::run_server;
use crate::daemon::remote::{
    SubmitAttemptStore, remote_node_context_from_env, remote_v1_bind_addr_from_env,
    run_remote_v1_http_server, run_remote_v1_tor_hidden_service,
    tor_hidden_service_runtime_config_from_env,
};

mod daemon;
mod paths;

pub use daemon::run_daemon;
pub use paths::{default_socket_path, default_state_db_path};
