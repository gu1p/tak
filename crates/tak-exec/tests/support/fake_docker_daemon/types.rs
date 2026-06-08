use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildRecord {
    pub image_tag: String,
    pub dockerfile: String,
    pub context_entries: Vec<String>,
    pub context_modes: BTreeMap<String, u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateRecord {
    pub user: Option<String>,
    pub binds: Vec<String>,
    pub nano_cpus: Option<i64>,
    pub memory: Option<i64>,
    pub memory_swap: Option<i64>,
    pub oom_kill_disable: Option<bool>,
    pub env: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoveRecord {
    pub container_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRecord {
    pub image: String,
}
