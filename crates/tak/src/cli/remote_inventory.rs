use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use tak_exec::record_remote_observation;
use tak_proto::{NodeInfo, decode_remote_token, decode_tor_invite};

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
    let mut inventory = load_inventory()?;
    let record = if token.trim().starts_with("takd:v1:") {
        let payload = decode_remote_token(token)?;
        let node = payload
            .node
            .ok_or_else(|| anyhow!("remote token is missing node info"))?;
        if node.transport == "tor" {
            bail!("tor onboarding now uses `takd:tor:` invites");
        }
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
        inventory
            .remotes
            .retain(|remote| remote.node_id != node.node_id);
        let _ = record_remote_observation(&probed);
        RemoteRecord {
            node_id: probed.node_id,
            display_name: probed.display_name,
            base_url: probed.base_url,
            bearer_token: payload.bearer_token,
            pools: probed.pools,
            tags: probed.tags,
            capabilities: probed.capabilities,
            transport: probed.transport,
            enabled: true,
        }
    } else if token.trim().starts_with("takd:tor:") {
        let base_url = decode_tor_invite(token)?;
        let probed = probe_node(&base_url, "tor", "")
            .await
            .with_context(|| format!("failed to probe remote node at {base_url} via tor"))?;
        ensure_tor_invite_matches_probe(&base_url, &probed)?;
        inventory
            .remotes
            .retain(|remote| remote.node_id != probed.node_id);
        let _ = record_remote_observation(&probed);
        RemoteRecord {
            node_id: probed.node_id,
            display_name: probed.display_name,
            base_url,
            bearer_token: String::new(),
            pools: probed.pools,
            tags: probed.tags,
            capabilities: probed.capabilities,
            transport: "tor".to_string(),
            enabled: true,
        }
    } else {
        bail!("remote invite must start with `takd:v1:` or `takd:tor:`");
    };
    inventory.remotes.push(record.clone());
    save_inventory(&inventory)?;
    Ok(record)
}

fn ensure_tor_invite_matches_probe(invited_base_url: &str, probed: &NodeInfo) -> Result<()> {
    if probed.transport == "tor" && probed.base_url == invited_base_url {
        return Ok(());
    }
    bail!(
        "tor invite expected {} via tor, probe returned {} via {}",
        invited_base_url,
        probed.base_url,
        probed.transport
    );
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
