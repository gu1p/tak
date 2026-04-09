//! Workspace discovery and `TASKS.py` loading.
//!
//! This crate discovers task definition files, evaluates them via Monty, converts output
//! into strongly-typed core models, and assembles a resolved `WorkspaceSpec`.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use monty::{LimitedTracker, MontyObject, MontyRun, PrintWriter, ResourceLimits};
use monty_type_checking::{SourceFile, type_check};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use tak_core::label::parse_label;
use tak_core::model::{
    CurrentStateDef, CurrentStateSpec, IgnoreSourceDef, IgnoreSourceSpec, LimiterDef, LimiterKey,
    LimiterRef, LocalSpec, ModuleSpec, PathInputDef, PolicyDecisionDef, PolicyDecisionModeDef,
    PolicyDecisionSpec, QueueDef, RemoteDef, RemoteRuntimeDef, RemoteRuntimeSpec, RemoteSpec,
    RemoteTransportDef, RemoteTransportKind, ResolvedTask, RetryDef, Scope, TaskExecutionDef,
    TaskExecutionSpec, TaskLabel, WorkspaceSpec, normalize_path_ref,
    validate_container_runtime_execution_spec,
};

const TASKS_FILE: &str = "TASKS.py";
const V1_TRANSPORT_DIRECT: &str = "direct";
const V1_TRANSPORT_TOR: &str = "tor";
const PRELUDE: &str = include_str!("loader/prelude.py");
const DSL_STUBS: &str = include_str!("loader/dsl_stubs.pyi");

include!("loader/load_options.rs");
include!("loader/workspace_discovery.rs");
include!("loader/workspace_load_and_policy_eval.rs");
include!("loader/project_resolution.rs");
include!("loader/module_merge.rs");
include!("loader/context_resolution.rs");
include!("loader/execution_resolution.rs");
include!("loader/remote_validation.rs");
include!("loader/module_eval.rs");
