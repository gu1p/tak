#![allow(dead_code)]

use std::fs;
use std::path::Path;

use anyhow::Result;
use serde::Serialize;

#[derive(Serialize)]
struct RemoteInventory<'a> {
    remotes: &'a [RemoteRecord],
}

#[derive(Serialize)]
pub struct RemoteRecord {
    pub node_id: String,
    pub display_name: String,
    pub base_url: String,
    pub bearer_token: String,
    pub pools: Vec<String>,
    pub tags: Vec<String>,
    pub capabilities: Vec<String>,
    pub transport: String,
    pub enabled: bool,
}

pub fn write_remote_inventory(config_root: &Path, remotes: &[RemoteRecord]) -> Result<()> {
    let inventory_root = config_root.join("tak");
    fs::create_dir_all(&inventory_root)?;
    fs::write(
        inventory_root.join("remotes.toml"),
        toml::to_string(&RemoteInventory { remotes })?,
    )?;
    Ok(())
}
