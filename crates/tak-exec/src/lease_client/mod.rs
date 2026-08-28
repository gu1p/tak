use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use tak_core::model::{NeedDef, ResolvedTask};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use uuid::Uuid;

use crate::{LeaseContext, RunOptions};

mod acquire_release;
#[cfg(test)]
mod acquire_release_tests;
mod coordination_status;
mod needs_transport;
mod wire_types;

#[cfg(test)]
mod wire_types_tests;

pub(crate) use acquire_release::{TaskLease, acquire_task_lease, release_task_lease};

use needs_transport::convert_needs;
use needs_transport::send_daemon_request;
use wire_types::NeedRequest;
use wire_types::{
    AcquireLeaseRequest, ClientInfo, LeaseInfo, ReleaseLeaseRequest, RenewLeaseRequest, Request,
    Response, TaskInfo,
};
