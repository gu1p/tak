use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteRecord {
    pub node_id: String,
    #[serde(default)]
    pub display_name: String,
    pub base_url: String,
    pub bearer_token: String,
    #[serde(default)]
    pub pools: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    pub transport: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteInventory {
    #[serde(default = "default_inventory_version")]
    pub version: u32,
    #[serde(default)]
    pub remotes: Vec<RemoteRecord>,
}

impl Default for RemoteInventory {
    fn default() -> Self {
        Self {
            version: default_inventory_version(),
            remotes: Vec::new(),
        }
    }
}

impl RemoteInventory {
    pub fn enabled_tor_remotes(&self) -> impl Iterator<Item = &RemoteRecord> {
        self.remotes
            .iter()
            .filter(|remote| remote.enabled && remote.transport == "tor")
    }
}

pub fn default_remote_inventory_path() -> anyhow::Result<PathBuf> {
    let root = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Ok(remote_inventory_path_from_config_home(&root))
}

pub fn remote_inventory_path_from_config_home(config_home: &Path) -> PathBuf {
    config_home.join("tak").join("remotes.toml")
}

pub fn load_remote_inventory_at(path: &Path) -> anyhow::Result<RemoteInventory> {
    if !path.exists() {
        return Ok(RemoteInventory::default());
    }
    let raw = fs::read_to_string(path)?;
    Ok(toml::from_str(&raw)?)
}

pub fn save_remote_inventory_at(path: &Path, inventory: &RemoteInventory) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, toml::to_string(inventory)?)?;
    Ok(())
}

fn default_inventory_version() -> u32 {
    1
}

fn default_enabled() -> bool {
    true
}
