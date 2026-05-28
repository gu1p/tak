use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use tak_core::model::Scope;
use tokio::io::{
    AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader,
};
use tokio::net::{UnixListener, UnixStream};

use crate::daemon::lease::{AcquireLeaseResponse, SharedLeaseManager};

mod broker;
mod daemon_tasks;
mod dispatch;
mod local_protocol_io;
mod request_wire;
mod types;
mod unix_server;
mod validation;

use broker::handle_broker_http_request;
use broker::{BrokerForwardResponse, BrokerRemoteHttpRequest};
use daemon_tasks::DaemonTaskHandles;
use dispatch::dispatch_request;
use local_protocol_io::handle_client;

pub use broker::TorBroker;
pub use types::{
    AcquireLeaseRequest, CancelTaskRequest, ClientInfo, ForwardRemoteHttpRequest,
    GetOutputRangeRequest, GetTaskResultRequest, LeaseInfo, LimiterUsage, NeedRequest,
    PeersEligibleRequest, PeersListRequest, PendingInfo, PlaceRemoteRequest, ReleaseLeaseRequest,
    RemoteResponseHeader, RenewLeaseRequest, Request, Response, StatusRequest, StatusSnapshot,
    StreamTaskEventsRequest, TaskInfo,
};
pub use unix_server::{run_server, run_server_with_broker, run_server_with_broker_and_peers};
pub use validation::ensure_valid_request;
