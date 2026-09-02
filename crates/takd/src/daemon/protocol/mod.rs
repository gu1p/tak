use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tak_core::model::Scope;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

use crate::daemon::lease::SharedLeaseManager;
use crate::daemon::run_store::RunStore;

mod broker;
mod local_protocol_io;
mod server_background_task;
mod types;
mod unix_server;
mod v2_dispatch;

#[cfg(test)]
mod server_background_task_tests;

use local_protocol_io::handle_client;

pub use broker::TorBroker;
pub use types::{
    AcquireLeaseRequest, ClientInfo, LeaseInfo, LimiterUsage, NeedRequest, PendingInfo,
    StatusSnapshot, TaskInfo,
};
pub use unix_server::{
    run_server_with_broker_and_peers, run_server_with_broker_peers_and_remote_inventory,
    run_server_with_broker_peers_and_run_store, run_server_with_local_attempt_executable,
    run_server_with_local_attempt_executable_and_remote_inventory_until_shutdown,
    run_server_with_local_attempt_executable_until_shutdown,
};
