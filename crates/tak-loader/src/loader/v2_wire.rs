use serde::Deserialize;

mod scheduling;

pub(super) use scheduling::{Backoff, Limiter, Need, QueueDefinition, QueueUse, Retry};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Module {
    #[serde(rename = "__tak_kind")]
    pub(super) kind: String,
    pub(super) spec_version: u32,
    pub(super) project_id: Option<String>,
    pub(super) tasks: Vec<Task>,
    pub(super) limiters: Vec<Limiter>,
    pub(super) queues: Vec<QueueDefinition>,
    pub(super) exclude: Vec<String>,
    pub(super) includes: Vec<Output>,
    pub(super) defaults: Defaults,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Defaults {
    #[serde(rename = "__tak_kind")]
    pub(super) kind: String,
    pub(super) queue: Option<QueueUse>,
    pub(super) retry: Option<Retry>,
    pub(super) container: Option<Unsupported>,
    pub(super) execution: Option<Execution>,
    pub(super) tags: Vec<String>,
    pub(super) pass_env: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Task {
    pub(super) name: String,
    pub(super) deps: Vec<String>,
    pub(super) steps: Vec<Step>,
    pub(super) needs: Vec<Need>,
    pub(super) queue: Option<QueueUse>,
    pub(super) retry: Option<Retry>,
    pub(super) timeout_s: Option<u64>,
    pub(super) context: Option<Unsupported>,
    pub(super) outputs: Vec<Output>,
    pub(super) execution: Option<Execution>,
    pub(super) session: Option<Box<Session>>,
    pub(super) cascade_session: bool,
    pub(super) tags: Vec<String>,
    pub(super) doc: String,
    pub(super) idempotent: bool,
    pub(super) pass_env: Vec<String>,
    pub(super) affinity: Option<Affinity>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Step {
    Cmd {
        argv: Vec<String>,
        cwd: Option<String>,
        env: std::collections::BTreeMap<String, String>,
    },
    Script {
        path: String,
        argv: Vec<String>,
        interpreter: Option<String>,
        cwd: Option<String>,
        env: std::collections::BTreeMap<String, String>,
    },
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Output {
    Path { value: String },
    Glob { value: String },
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Execution {
    LocalOnly { local: Local },
    RemoteOnly { remote: Remote },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Local {
    pub(super) container: Option<Unsupported>,
    pub(super) session: Option<Box<Session>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Remote {
    pub(super) pool: Option<String>,
    pub(super) required_tags: Vec<String>,
    pub(super) required_capabilities: Vec<String>,
    pub(super) transport: Option<Transport>,
    pub(super) container: Option<Unsupported>,
    pub(super) selection: Selection,
    pub(super) session: Option<Box<Session>>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Selection {
    Balanced,
    Sequential,
    RoundRobin,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Transport {
    Direct,
    Any,
    Tor,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Session {
    #[serde(rename = "__tak_kind")]
    pub(super) kind: String,
    pub(super) id: String,
    pub(super) name: Option<String>,
    pub(super) execution: Option<Box<Execution>>,
    pub(super) reuse: Reuse,
    pub(super) context: Option<Unsupported>,
    pub(super) affinity: Option<Affinity>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Reuse {
    Workspace,
    Paths { paths: Vec<Output> },
    SharedWorkspace { max_parallel_tasks: u32 },
    Container,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Affinity {
    PreferSameNode { group: String },
    RequireSameNode { group: String },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Unsupported {}
