use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildRecord {
    pub dockerfile: String,
    pub context_entries: Vec<String>,
    pub context_modes: BTreeMap<String, u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateRecord {
    pub user: Option<String>,
    pub binds: Vec<String>,
}
