use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow};
use rusqlite::{Connection, params};
use tak_core::model::Scope;
use uuid::Uuid;

use crate::daemon::protocol::{
    AcquireLeaseRequest, LeaseInfo, LimiterUsage, NeedRequest, PendingInfo, StatusSnapshot,
};

mod manager_allocation_methods;
mod manager_persistence_load;
mod manager_persistence_store;
mod manager_public_methods;
mod manager_setup_and_status;
mod models;
mod shared;

use models::{LeaseRecord, LimiterKey};
use shared::StoredLeaseRow;

pub use models::{AcquireLeaseResponse, LeaseManager};
pub use shared::{SharedLeaseManager, new_shared_manager, new_shared_manager_with_db};

fn unix_epoch_ms() -> i64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    i64::try_from(millis).unwrap_or(i64::MAX)
}
