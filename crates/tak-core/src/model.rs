//! Canonical model types shared by all Tak crates.
//!
//! These structures represent loader output, execution plans, limiter references, and
//! runtime workspace state.

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

include!("model/task_identity.rs");
include!("model/module_spec.rs");
include!("model/remote_config.rs");
include!("model/container_runtime_types.rs");
include!("model/execution_policy.rs");
include!("model/limiter_retry.rs");
include!("model/resolved_workspace.rs");
include!("model/container_runtime_validation.rs");
include!("model/context_manifest.rs");
include!("model/current_state_manifest.rs");
include!("model/path_anchor.rs");
include!("model/container_runtime_normalization.rs");
include!("model/container_runtime_limits.rs");
include!("model/relative_path.rs");
