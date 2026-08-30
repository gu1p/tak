use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::loader) struct ScopedReference {
    pub(in crate::loader) name: String,
    pub(in crate::loader) scope: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::loader) struct Need {
    pub(in crate::loader) limiter: ScopedReference,
    pub(in crate::loader) slots: f64,
    pub(in crate::loader) hold: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::loader) struct QueueUse {
    pub(in crate::loader) queue: ScopedReference,
    pub(in crate::loader) slots: i32,
    pub(in crate::loader) priority: i32,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(in crate::loader) enum Limiter {
    Resource {
        name: String,
        scope: String,
        capacity: f64,
        unit: Option<String>,
    },
    Lock {
        name: String,
        scope: String,
    },
    RateLimit {
        name: String,
        scope: String,
        burst: u32,
        refill_per_second: f64,
    },
    ProcessCap {
        name: String,
        scope: String,
        max_running: u32,
        #[serde(rename = "match")]
        match_pattern: Option<String>,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::loader) struct QueueDefinition {
    pub(in crate::loader) name: String,
    pub(in crate::loader) scope: String,
    pub(in crate::loader) slots: u32,
    pub(in crate::loader) discipline: String,
    pub(in crate::loader) max_pending: Option<u32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::loader) struct Retry {
    pub(in crate::loader) attempts: u32,
    pub(in crate::loader) on_exit: Vec<i32>,
    pub(in crate::loader) backoff: Backoff,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(in crate::loader) enum Backoff {
    Fixed {
        seconds: f64,
    },
    ExpJitter {
        min_s: f64,
        max_s: f64,
        jitter: String,
    },
}
