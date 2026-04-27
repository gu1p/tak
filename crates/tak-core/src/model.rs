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

mod container_runtime_limits;
mod container_runtime_normalization;
mod container_runtime_types;
mod container_runtime_validation;
mod context_manifest;
mod current_state_manifest;
mod execution_policy;
mod limiter_retry;
mod module_spec;
mod path_anchor;
mod relative_path;
mod remote_config;
mod resolved_execution;
mod resolved_session;
mod resolved_workspace;
mod task_identity;

pub use self::container_runtime_types::{
    ContainerImageReference, ContainerImageReferenceError, ContainerMountSpec,
    ContainerResourceLimitsSpec, ContainerRuntimeExecutionSpec, ContainerRuntimeExecutionSpecError,
    ContainerRuntimeSourceInputSpec,
};
pub use self::container_runtime_validation::{
    normalize_container_image_reference, validate_container_runtime_execution_spec,
};
pub use self::context_manifest::normalize_path_ref;
pub use self::current_state_manifest::build_current_state_manifest;
pub use self::execution_policy::{
    ExecutionPolicyDef, Hold, NeedDef, PolicyDecisionDef, PolicyDecisionModeDef, QueueUseDef,
    StepDef, TaskExecutionDef,
};
pub use self::limiter_retry::{BackoffDef, LimiterDef, QueueDef, QueueDiscipline, RetryDef};
pub use self::module_spec::{
    CurrentStateDef, Defaults, IgnoreSourceDef, LocalDef, ModuleSpec, OutputSelectorDef,
    PathInputDef, SessionDef, SessionReuseDef, TaskDef,
};
pub use self::path_anchor::PathNormalizationError;
pub use self::remote_config::{
    ContainerMountDef, ContainerResourceLimitsDef, RemoteDef, RemoteRuntimeDef, RemoteSelectionDef,
    RemoteTransportDef, RemoteTransportKind,
};
pub use self::resolved_execution::{
    ContainerRuntimeSourceSpec, ExecutionPlacementSpec, ExecutionPolicySpec, LocalSpec,
    PolicyDecisionSpec, RemoteRuntimeSpec, RemoteSelectionSpec, RemoteSpec, TaskExecutionSpec,
};
pub use self::resolved_session::{
    SessionLifetimeSpec, SessionReuseSpec, SessionSpec, SessionUseSpec,
};
pub use self::resolved_workspace::{
    ContextManifest, CurrentStateOrigin, CurrentStateSpec, IgnoreSourceSpec, LimiterKey,
    OutputSelectorSpec, PathAnchor, PathRef, ResolvedTask, WorkspaceSpec,
};
pub use self::task_identity::{LimiterRef, Scope, TaskLabel};

pub(crate) use self::container_runtime_limits::{
    is_sensitive_runtime_env_key, normalize_runtime_resource_limits,
};
pub(crate) use self::container_runtime_normalization::{
    normalize_image_name_and_tag, normalize_runtime_command, normalize_runtime_env,
    normalize_runtime_mounts,
};
pub(crate) use self::current_state_manifest::{compare_path_ref, hash_manifest_entries};
pub(crate) use self::module_spec::default_local_parallelism;
pub(crate) use self::path_anchor::parse_anchor;
pub(crate) use self::relative_path::normalize_relative_path;
