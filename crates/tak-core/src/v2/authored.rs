use std::collections::BTreeMap;
use std::fmt::{Debug, Formatter};

use serde::{Deserialize, Serialize};

use super::{Affinity, Execution, PassEnv};

mod scheduling;

pub use scheduling::{
    AuthoredLimiterClaim, AuthoredLimiterDefinition, AuthoredQueueDefinition, AuthoredQueueUse,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OutputSelector {
    Path { value: String },
    Glob { value: String },
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Step {
    Cmd {
        argv: Vec<String>,
        cwd: Option<String>,
        env: BTreeMap<String, String>,
    },
    Script {
        path: String,
        argv: Vec<String>,
        interpreter: Option<String>,
        cwd: Option<String>,
        env: BTreeMap<String, String>,
    },
}

impl Debug for Step {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cmd { argv, cwd, env } => formatter
                .debug_struct("Cmd")
                .field("argv", argv)
                .field("cwd", cwd)
                .field("env", &redacted_env(env))
                .finish(),
            Self::Script {
                path,
                argv,
                interpreter,
                cwd,
                env,
            } => formatter
                .debug_struct("Script")
                .field("path", path)
                .field("argv", argv)
                .field("interpreter", interpreter)
                .field("cwd", cwd)
                .field("env", &redacted_env(env))
                .finish(),
        }
    }
}

fn redacted_env(env: &BTreeMap<String, String>) -> BTreeMap<&str, &str> {
    env.keys()
        .map(|name| (name.as_str(), "<redacted>"))
        .collect()
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredDefaults {
    pub execution: Option<Execution>,
    pub retry: Option<super::RetryPolicy>,
    pub queue: Option<AuthoredQueueUse>,
    pub pass_env: PassEnv,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskContext {
    pub roots: Vec<String>,
    pub ignored_paths: Vec<String>,
    pub use_gitignore: bool,
    pub include: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredTask {
    pub name: String,
    pub doc: String,
    pub deps: Vec<String>,
    pub steps: Vec<Step>,
    pub outputs: Vec<OutputSelector>,
    pub context: Option<TaskContext>,
    pub execution: Option<Execution>,
    pub retry: Option<super::RetryPolicy>,
    pub queue: Option<AuthoredQueueUse>,
    pub limiter_claims: Vec<AuthoredLimiterClaim>,
    pub session: Option<super::Session>,
    pub cascade_session: bool,
    pub idempotent: bool,
    pub pass_env: PassEnv,
    pub affinity: Option<Affinity>,
    pub tags: Vec<String>,
    pub timeout_s: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredModule {
    pub project_id: Option<String>,
    pub tasks: Vec<AuthoredTask>,
    pub limiter_definitions: Vec<AuthoredLimiterDefinition>,
    pub queue_definitions: Vec<AuthoredQueueDefinition>,
    pub includes: Vec<OutputSelector>,
    pub exclude: Vec<String>,
    pub defaults: AuthoredDefaults,
}
