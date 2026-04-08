use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use tak_proto::decode_remote_token;

use super::remote_probe::probe_node;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct RemoteRecord {
    pub(super) node_id: String,
    pub(super) display_name: String,
    pub(super) base_url: String,
    pub(super) bearer_token: String,
    pub(super) pools: Vec<String>,
    pub(super) tags: Vec<String>,
    pub(super) capabilities: Vec<String>,
    pub(super) transport: String,
    pub(super) enabled: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct RemoteInventoryFile {
    version: u32,
    remotes: Vec<RemoteRecord>,
}

pub(super) async fn add_remote(token: &str) -> Result<RemoteRecord> {
    let payload = decode_remote_token(token)?;
    let node = payload
        .node
        .ok_or_else(|| anyhow!("remote token is missing node info"))?;
    let probed = probe_node(&node.base_url, &node.transport, &payload.bearer_token)
        .await
        .with_context(|| {
            format!(
                "failed to probe remote node {} at {} via {}",
                node.node_id, node.base_url, node.transport
            )
        })?;
    if probed.node_id != node.node_id {
        bail!(
            "remote node id mismatch: token={}, probe={}",
            node.node_id,
            probed.node_id
        );
    }

    let mut inventory = load_inventory()?;
    inventory
        .remotes
        .retain(|remote| remote.node_id != node.node_id);
    let record = RemoteRecord {
        node_id: node.node_id,
        display_name: node.display_name,
        base_url: node.base_url,
        bearer_token: payload.bearer_token,
        pools: node.pools,
        tags: node.tags,
        capabilities: node.capabilities,
        transport: node.transport,
        enabled: true,
    };
    inventory.remotes.push(record.clone());
    save_inventory(&inventory)?;
    Ok(record)
}

pub(super) fn list_remotes() -> Result<Vec<RemoteRecord>> {
    Ok(load_inventory()?.remotes)
}

pub(super) fn remove_remote(node_id: &str) -> Result<bool> {
    let mut inventory = load_inventory()?;
    let before = inventory.remotes.len();
    inventory.remotes.retain(|remote| remote.node_id != node_id);
    save_inventory(&inventory)?;
    Ok(inventory.remotes.len() != before)
}

fn inventory_path() -> Result<PathBuf> {
    let root = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|home| PathBuf::from(home).join(".config")))
        .map_err(|_| anyhow!("failed to resolve config home"))?;
    Ok(root.join("tak").join("remotes.toml"))
}

fn load_inventory() -> Result<RemoteInventoryFile> {
    let path = inventory_path()?;
    if !path.exists() {
        return Ok(RemoteInventoryFile {
            version: 1,
            remotes: Vec::new(),
        });
    }
    let raw = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    toml::from_str(&raw).with_context(|| format!("decode {}", path.display()))
}

fn save_inventory(inventory: &RemoteInventoryFile) -> Result<()> {
    let path = inventory_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        &path,
        toml::to_string(inventory).context("encode remote inventory")?,
    )
    .with_context(|| format!("write {}", path.display()))?;
    Ok(())
}
