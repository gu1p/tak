use std::fs;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use tak_proto::{RemoteTokenPayload, decode_remote_token, encode_remote_token};
use uuid::Uuid;

use crate::daemon::remote::RemoteNodeContext;
use helpers::{hidden_service_nickname, node_info, normalize_values};
use paths::{config_path, token_path};

const CONFIG_FILE: &str = "agent.toml";
const TOKEN_FILE: &str = "agent.token";

mod direct_base_url;
mod helpers;
mod paths;
mod transport_health;

pub(crate) use direct_base_url::{DirectBaseUrlError, parse_direct_base_url};
pub(crate) use helpers::node_info_with_transport;
pub use paths::{arti_cache_dir, arti_state_dir, default_config_root, default_state_root};
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
}

pub struct InitAgentOptions<'a> {
    pub node_id: Option<&'a str>,
    pub display_name: Option<&'a str>,
    pub transport: Option<&'a str>,
    pub base_url: Option<&'a str>,
    pub pools: &'a [String],
    pub tags: &'a [String],
    pub capabilities: &'a [String],
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
    let token =
        fs::read_to_string(token_path(state_root)).map_err(|_| anyhow!("agent token not ready"))?;
    let token = token.trim().to_string();
    if token.is_empty() {
        bail!("agent token not ready");
    }
    let _ = decode_remote_token(&token)?;
    Ok(token)
}

pub fn read_token_wait(state_root: &Path, timeout_secs: u64) -> Result<String> {
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        match read_token(state_root) {
            Ok(token) => return Ok(token),
            Err(err) if err.to_string().contains("not ready") && Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(100));
            }
            Err(err) => return Err(err),
        }
    }
}

pub fn persist_ready_base_url(
    config_root: &Path,
    state_root: &Path,
    base_url: &str,
) -> Result<String> {
    let mut config = read_config(config_root)?;
    let base_url = base_url.trim();
    if base_url.is_empty() {
        bail!("base_url is required");
    }
    config.base_url = Some(base_url.to_string());
    write_config(config_root, &config)?;
    let token = encode_remote_token(&RemoteTokenPayload {
        version: "v1".to_string(),
        node: Some(node_info(&config, base_url)),
        bearer_token: config.bearer_token.clone(),
    })?;
    fs::write(token_path(state_root), format!("{token}\n"))?;
    Ok(token)
}

pub fn ready_context(config: &AgentConfig) -> Result<RemoteNodeContext> {
    let base_url = config
        .base_url
        .clone()
        .ok_or_else(|| anyhow!("agent token not ready"))?;
    Ok(RemoteNodeContext::new(
        node_info(config, &base_url),
        config.bearer_token.clone(),
    ))
}

fn write_config(config_root: &Path, config: &AgentConfig) -> Result<()> {
    fs::write(
        config_path(config_root),
        toml::to_string(config).context("encode agent config")?,
    )?;
    Ok(())
}

fn validate_direct_base_url(base_url: Option<&str>) -> Result<String> {
    parse_direct_base_url(base_url)
        .map(|parsed| parsed.canonical_base_url())
        .map_err(|err| match err {
            DirectBaseUrlError::Missing => anyhow!("base_url is required for direct transport"),
            DirectBaseUrlError::InvalidScheme => {
                anyhow!("base_url must start with http:// or https:// for direct transport")
            }
            DirectBaseUrlError::MissingHost => {
                anyhow!("base_url must include a host for direct transport")
            }
            DirectBaseUrlError::MissingPort => {
                anyhow!("base_url must include a port for direct transport")
            }
            DirectBaseUrlError::UnsupportedComponents => anyhow!(
                "base_url must not include userinfo, path, query, or fragment for direct transport"
            ),
        })
}
