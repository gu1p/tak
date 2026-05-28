use std::path::PathBuf;

use anyhow::{Context, Result, anyhow, bail};
use tak_core::remote_inventory::{
    RemoteInventory, default_remote_inventory_path, load_remote_inventory_at,
    save_remote_inventory_at,
};
use tak_exec::record_remote_observation;
use tak_proto::{NodeInfo, decode_remote_token, decode_tor_invite_payload};

use super::remote_probe::probe_node;

pub(in crate::cli) use tak_core::remote_inventory::RemoteRecord;

pub(super) async fn add_remote(token: &str) -> Result<RemoteRecord> {
    let record = resolve_remote_record(token).await?;
    save_remote_record(&record)?;
    Ok(record)
}

pub(super) async fn resolve_remote_record(token: &str) -> Result<RemoteRecord> {
    if token.trim().starts_with("takd:v1:") {
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
        let _ = record_remote_observation(&probed);
        Ok(RemoteRecord {
            node_id: probed.node_id,
            display_name: probed.display_name,
            base_url: probed.base_url,
            bearer_token: payload.bearer_token,
            pools: probed.pools,
            tags: probed.tags,
            capabilities: probed.capabilities,
            transport: probed.transport,
            enabled: true,
        })
    } else if token.trim().starts_with("takd:tor:") {
        let invite = decode_tor_invite_payload(token)?;
        let probed = probe_node(&invite.base_url, "tor", &invite.bearer_token)
            .await
            .with_context(|| {
                format!("failed to probe remote node at {} via tor", invite.base_url)
            })?;
        ensure_tor_invite_matches_probe(&invite.base_url, &probed)?;
        let _ = record_remote_observation(&probed);
        Ok(RemoteRecord {
            node_id: probed.node_id,
            display_name: probed.display_name,
            base_url: invite.base_url,
            bearer_token: invite.bearer_token,
            pools: probed.pools,
            tags: probed.tags,
            capabilities: probed.capabilities,
            transport: "tor".to_string(),
            enabled: true,
        })
    } else {
        bail!("remote invite must start with `takd:v1:` or `takd:tor:`");
    }
}

pub(super) fn save_remote_record(record: &RemoteRecord) -> Result<()> {
    let mut inventory = load_inventory()?;
    inventory
        .remotes
        .retain(|remote| remote.node_id != record.node_id);
    inventory.remotes.push(record.clone());
    save_inventory(&inventory)?;
    Ok(())
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
    default_remote_inventory_path().map_err(|_| anyhow!("failed to resolve config home"))
}

fn load_inventory() -> Result<RemoteInventory> {
    let path = inventory_path()?;
    load_remote_inventory_at(&path).with_context(|| format!("decode {}", path.display()))
}

fn save_inventory(inventory: &RemoteInventory) -> Result<()> {
    let path = inventory_path()?;
    save_remote_inventory_at(&path, inventory)
        .with_context(|| format!("write {}", path.display()))?;
    Ok(())
}
