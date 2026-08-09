use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use tak_core::model::Scope;

use crate::daemon::lease::new_shared_manager_with_db;

mod daemon;
mod paths;

pub(crate) use daemon::run_local_daemon_with_broker_and_peers;
pub use paths::default_socket_path;
