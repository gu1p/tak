use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{Affinity, Execution, PassEnv};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OutputSelector {
    Path { value: String },
    Glob { value: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredDefaults {
    pub execution: Option<Execution>,
    pub pass_env: PassEnv,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredTask {
    pub name: String,
    pub doc: String,
    pub deps: Vec<String>,
    pub steps: Vec<Step>,
    pub outputs: Vec<OutputSelector>,
    pub execution: Option<Execution>,
    pub idempotent: bool,
    pub pass_env: PassEnv,
    pub affinity: Option<Affinity>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredModule {
    pub project_id: Option<String>,
    pub tasks: Vec<AuthoredTask>,
    pub includes: Vec<OutputSelector>,
    pub exclude: Vec<String>,
    pub defaults: AuthoredDefaults,
}
