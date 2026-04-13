use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};
use arti_client::TorClientConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ClientTorPaths {
    state_root: PathBuf,
    arti_state_dir: PathBuf,
    arti_cache_dir: PathBuf,
}

pub fn default_client_tor_config() -> Result<TorClientConfig> {
    let paths = resolve_client_tor_paths()?;
    arti_client::config::TorClientConfigBuilder::from_directories(
        &paths.arti_state_dir,
        &paths.arti_cache_dir,
    )
    .build()
    .context("build tak client tor config")
}

fn resolve_client_tor_paths() -> Result<ClientTorPaths> {
    let state_root = env_path("XDG_STATE_HOME")
        .map(|path| path.join("tak"))
        .or_else(|| env_path("HOME").map(|path| path.join(".local/state").join("tak")))
        .ok_or_else(|| anyhow!("failed to resolve tak client state root"))?;
    Ok(ClientTorPaths {
        arti_state_dir: state_root.join("arti").join("state"),
        arti_cache_dir: state_root.join("arti").join("cache"),
        state_root,
    })
}

fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}
