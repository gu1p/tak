use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use tak_core::label::parse_label;
use tak_core::model::Scope;
use tak_exec::{RunOptions, run_tasks};
use tak_loader::{LoadOptions, load_workspace};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

use crate::daemon::lease::{AcquireLeaseResponse, SharedLeaseManager};

mod dispatch;
mod local_protocol_io;
mod run_tasks_request;
mod types;
mod unix_server;
mod validation;

use dispatch::dispatch_request;
use local_protocol_io::handle_client;
use run_tasks_request::execute_run_tasks_request;

pub use types::{
    AcquireLeaseRequest, ClientInfo, LeaseInfo, LimiterUsage, NeedRequest, PendingInfo,
    ReleaseLeaseRequest, RenewLeaseRequest, Request, Response, RunTasksRequest, StatusRequest,
    StatusSnapshot, TaskInfo,
};
pub use unix_server::run_server;
pub use validation::ensure_valid_request;
