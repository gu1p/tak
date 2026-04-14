use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tak_proto::NodeInfo;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteObservation {
    pub node_id: String,
    pub sampled_at_ms: i64,
    pub transport: String,
    pub healthy: bool,
    pub transport_state: String,
    pub transport_detail: String,
}

pub fn record_remote_observation(node: &NodeInfo) -> Result<()> {
    write_remote_observation(node, unix_epoch_ms())
}

pub fn write_remote_observation(node: &NodeInfo, sampled_at_ms: i64) -> Result<()> {
    write_remote_observation_at(&state_home()?, node, sampled_at_ms)
}

pub fn write_remote_observation_at(
    state_home: &Path,
    node: &NodeInfo,
    sampled_at_ms: i64,
) -> Result<()> {
    let path = observation_path(state_home, &node.node_id);
    let observation = RemoteObservation {
        node_id: node.node_id.clone(),
        sampled_at_ms,
        transport: node.transport.clone(),
        healthy: node.healthy,
        transport_state: node.transport_state.clone(),
        transport_detail: node.transport_detail.clone(),
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let temp_path = path.with_extension(format!("{}.tmp", Uuid::new_v4().simple()));
    fs::write(
        &temp_path,
        toml::to_string(&observation).context("encode remote observation")?,
    )
    .with_context(|| format!("write {}", temp_path.display()))?;
    fs::rename(&temp_path, &path).with_context(|| {
        format!(
            "move remote observation {} -> {}",
            temp_path.display(),
            path.display()
        )
    })?;
    Ok(())
}

pub fn load_remote_observation(node_id: &str) -> Result<Option<RemoteObservation>> {
    load_remote_observation_at(&state_home()?, node_id)
}

pub fn load_remote_observation_at(
    state_home: &Path,
    node_id: &str,
) -> Result<Option<RemoteObservation>> {
    let path = observation_path(state_home, node_id);
    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err).with_context(|| format!("read {}", path.display())),
    };
    Ok(toml::from_str::<RemoteObservation>(&raw)
        .ok()
        .filter(|observation| observation.node_id == node_id))
}

fn state_home() -> Result<PathBuf> {
    std::env::var("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .map_err(|_| anyhow!("failed to resolve state home"))
}

fn observation_path(state_home: &Path, node_id: &str) -> PathBuf {
    state_home
        .join("tak")
        .join("remote-observations")
        .join(format!("{}.toml", node_id_hash(node_id)))
}

fn node_id_hash(node_id: &str) -> String {
    Sha256::digest(node_id.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn unix_epoch_ms() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_millis() as i64,
        Err(_) => 0,
    }
}
