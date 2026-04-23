use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use tak_core::model::Scope;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

use crate::daemon::lease::{AcquireLeaseResponse, SharedLeaseManager};

mod dispatch;
mod local_protocol_io;
mod request_wire;
mod types;
mod unix_server;
mod validation;

use dispatch::dispatch_request;
use local_protocol_io::handle_client;

pub use types::{
    AcquireLeaseRequest, ClientInfo, LeaseInfo, LimiterUsage, NeedRequest, PendingInfo,
    ReleaseLeaseRequest, RenewLeaseRequest, Request, Response, StatusRequest, StatusSnapshot,
    TaskInfo,
};
pub use unix_server::run_server;
pub use validation::ensure_valid_request;
