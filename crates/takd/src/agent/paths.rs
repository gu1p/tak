use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};

use super::{CONFIG_FILE, TOKEN_FILE};

pub fn config_path(config_root: &Path) -> PathBuf {
    config_root.join(CONFIG_FILE)
}

pub fn token_path(state_root: &Path) -> PathBuf {
    state_root.join(TOKEN_FILE)
}

pub fn transport_health_path(state_root: &Path) -> PathBuf {
    state_root.join("transport-health.toml")
}

pub fn default_config_root() -> Result<PathBuf> {
    Ok(xdg_root("XDG_CONFIG_HOME", ".config")?.join("takd"))
}

pub fn default_state_root() -> Result<PathBuf> {
    Ok(xdg_root("XDG_STATE_HOME", ".local/state")?.join("takd"))
}

pub fn arti_state_dir(state_root: &Path) -> PathBuf {
    state_root.join("arti").join("state")
}

pub fn arti_cache_dir(state_root: &Path) -> PathBuf {
    state_root.join("arti").join("cache")
}

fn xdg_root(var: &str, fallback: &str) -> Result<PathBuf> {
    std::env::var(var)
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|home| PathBuf::from(home).join(fallback)))
        .map_err(|_| anyhow!("failed to resolve {}", var.to_ascii_lowercase()))
}
