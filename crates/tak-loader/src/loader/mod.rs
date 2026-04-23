use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use tak_core::model::{LimiterDef, LimiterKey, QueueDef, ResolvedTask, TaskLabel};

mod authored_source;
mod context_resolution;
mod execution_resolution;
mod load_options;
mod module_eval;
mod module_merge;
mod monty_deserializer;
mod output_resolution;
mod project_resolution;
mod remote_validation;
mod workspace_discovery;
mod workspace_load_and_policy_eval;

pub use load_options::LoadOptions;
pub use workspace_discovery::{detect_workspace_root, discover_tasks_files};
pub use workspace_load_and_policy_eval::{evaluate_named_policy_decision, load_workspace};

const TASKS_FILE: &str = "TASKS.py";
const V1_TRANSPORT_ANY: &str = "any";
const V1_TRANSPORT_DIRECT: &str = "direct";
const V1_TRANSPORT_TOR: &str = "tor";
const PRELUDE: &str = include_str!("prelude.py");
const DSL_STUBS: &str = include_str!("dsl_stubs.pyi");

#[derive(Default)]
pub(crate) struct MergeState {
    pub(crate) tasks: BTreeMap<TaskLabel, ResolvedTask>,
    pub(crate) task_origins: BTreeMap<TaskLabel, PathBuf>,
    pub(crate) limiters: HashMap<LimiterKey, LimiterDef>,
    pub(crate) limiter_origins: HashMap<LimiterKey, PathBuf>,
    pub(crate) queues: HashMap<LimiterKey, QueueDef>,
    pub(crate) queue_origins: HashMap<LimiterKey, PathBuf>,
}
