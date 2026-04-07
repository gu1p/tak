#![allow(dead_code)]

use std::fs;
use std::path::Path;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct RemoteInventoryRecord {
    pub node_id: String,
    pub base_url: String,
    pub bearer_token: String,
    pub pools: Vec<String>,
    pub tags: Vec<String>,
    pub capabilities: Vec<String>,
    pub transport: String,
    pub enabled: bool,
}

impl RemoteInventoryRecord {
    pub fn builder(node_id: &str, base_url: &str, bearer_token: &str, transport: &str) -> Self {
        Self {
            node_id: node_id.to_string(),
            base_url: base_url.to_string(),
            bearer_token: bearer_token.to_string(),
            pools: vec!["build".to_string()],
            tags: vec!["builder".to_string()],
            capabilities: vec!["linux".to_string()],
            transport: transport.to_string(),
            enabled: true,
        }
    }
}

pub fn write_remote_inventory(config_root: &Path, records: &[RemoteInventoryRecord]) {
    #[derive(Serialize)]
    struct Inventory<'a> {
        remotes: &'a [RemoteInventoryRecord],
    }

    let path = config_root.join("tak").join("remotes.toml");
    fs::create_dir_all(path.parent().expect("inventory parent")).expect("create inventory parent");
    let body = toml::to_string(&Inventory { remotes: records }).expect("encode inventory");
    fs::write(path, body).expect("write remote inventory");
}
