use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use tak_proto::{RemoteTokenPayload, encode_remote_token, encode_tor_invite_with_bearer};
use uuid::Uuid;

use helpers::{hidden_service_nickname, node_info, normalize_values};
use paths::{config_path, token_path};
use token_state::read_token_error_into_anyhow;

const CONFIG_FILE: &str = "agent.toml";
const TOKEN_FILE: &str = "agent.token";

mod auto_update_config;
mod context;
mod direct_base_url;
mod helpers;
mod image_cache_config;
mod paths;
#[path = "agent/token_readiness_tests.rs"]
mod token_readiness_tests;
mod token_state;
mod token_wait;
mod transport_health;

pub use auto_update_config::{AutoUpdateConfig, UpdateNetwork};
pub use context::{ready_context, ready_context_with_state_root};
pub(crate) use direct_base_url::{
    DirectBaseUrlError, parse_direct_base_url, validate_direct_base_url,
};
pub(crate) use helpers::node_info_with_transport;
use image_cache_config::resolve_init_image_cache_config;
pub use image_cache_config::{AgentImageCacheConfig, interactive_image_cache_budget_gb};
pub use paths::{arti_cache_dir, arti_state_dir, default_config_root, default_state_root};
pub use token_wait::read_token_wait;
pub use transport_health::{
    TorRecoveryBackoff, TorRecoveryTracker, TransportHealth, TransportState, read_transport_health,
    write_transport_health,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub node_id: String,
    pub display_name: String,
    pub base_url: Option<String>,
    pub bearer_token: String,
    pub pools: Vec<String>,
    pub tags: Vec<String>,
    pub capabilities: Vec<String>,
    pub transport: String,
    pub hidden_service_nickname: String,
    #[serde(default)]
    pub image_cache: Option<AgentImageCacheConfig>,
    #[serde(default)]
    pub auto_update: AutoUpdateConfig,
}

pub struct InitAgentOptions<'a> {
    pub node_id: Option<&'a str>,
    pub display_name: Option<&'a str>,
    pub transport: Option<&'a str>,
    pub base_url: Option<&'a str>,
    pub pools: &'a [String],
    pub tags: &'a [String],
    pub capabilities: &'a [String],
    pub image_cache_budget_percent: Option<f64>,
    pub image_cache_budget_gb: Option<f64>,
}

pub fn init_agent(
    config_root: &Path,
    state_root: &Path,
    options: InitAgentOptions<'_>,
) -> Result<()> {
    fs::create_dir_all(config_root)?;
    fs::create_dir_all(state_root)?;
    if config_path(config_root).exists() {
        return Ok(());
    }

    let node_id = options
        .node_id
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("node-{}", Uuid::new_v4().simple()));
    let display_name = options
        .display_name
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| node_id.clone());
    let transport = options
        .transport
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("tor");
    let base_url = match transport {
        "tor" => None,
        "direct" => Some(validate_direct_base_url(options.base_url)?),
        other => bail!("unsupported takd transport `{other}`"),
    };
    let config = AgentConfig {
        node_id: node_id.clone(),
        display_name: display_name.clone(),
        base_url,
        bearer_token: Uuid::new_v4().to_string(),
        pools: normalize_values(options.pools, "default"),
        tags: normalize_values(options.tags, "builder"),
        capabilities: normalize_values(options.capabilities, "linux"),
        transport: transport.to_string(),
        hidden_service_nickname: hidden_service_nickname(&node_id),
        image_cache: Some(resolve_init_image_cache_config(state_root, &options)?),
        auto_update: AutoUpdateConfig::default(),
    };
    write_config(config_root, &config)?;
    let token_path = token_path(state_root);
    if token_path.exists() {
        fs::remove_file(token_path)?;
    }
    Ok(())
}

pub fn read_config(config_root: &Path) -> Result<AgentConfig> {
    let path = config_path(config_root);
    let raw = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    toml::from_str(&raw).with_context(|| format!("decode {}", path.display()))
}

pub fn read_token(state_root: &Path) -> Result<String> {
    token_wait::read_token_once_unless_transport_is_reported_unready(state_root)
        .map_err(read_token_error_into_anyhow)
}

pub fn persist_ready_base_url(
    config_root: &Path,
    state_root: &Path,
    base_url: &str,
) -> Result<String> {
    persist_advertised_base_url(config_root, base_url)?;
    let config = read_config(config_root)?;
    let base_url = base_url.trim();
    let token = if config.transport == "tor" {
        encode_tor_invite_with_bearer(base_url, &config.bearer_token)?
    } else {
        encode_remote_token(&RemoteTokenPayload {
            version: "v1".to_string(),
            node: Some(node_info(&config, base_url)),
            bearer_token: config.bearer_token.clone(),
        })?
    };
    fs::write(token_path(state_root), format!("{token}\n"))?;
    Ok(token)
}

pub fn persist_advertised_base_url(config_root: &Path, base_url: &str) -> Result<()> {
    let mut config = read_config(config_root)?;
    let base_url = base_url.trim();
    if base_url.is_empty() {
        bail!("base_url is required");
    }
    config.base_url = Some(base_url.to_string());
    write_config(config_root, &config)
}

fn write_config(config_root: &Path, config: &AgentConfig) -> Result<()> {
    fs::write(
        config_path(config_root),
        toml::to_string(config).context("encode agent config")?,
    )?;
    Ok(())
}
