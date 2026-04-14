use tak_proto::NodeInfo;
use uuid::Uuid;

use super::AgentConfig;

pub fn hidden_service_nickname(node_id: &str) -> String {
    let mut nickname = node_id
        .chars()
        .filter(|value| value.is_ascii_alphanumeric() || *value == '-')
        .collect::<String>();
    if nickname.is_empty() {
        nickname = format!("takd{}", Uuid::new_v4().simple());
    }
    nickname.truncate(32);
    nickname
}

pub fn normalize_values(values: &[String], default: &str) -> Vec<String> {
    let normalized = values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if normalized.is_empty() {
        return vec![default.to_string()];
    }
    normalized
}

pub fn node_info(config: &AgentConfig, base_url: &str) -> NodeInfo {
    node_info_with_transport(config, base_url, "ready", None)
}

pub fn node_info_with_transport(
    config: &AgentConfig,
    base_url: &str,
    transport_state: &str,
    transport_detail: Option<&str>,
) -> NodeInfo {
    let transport_state = if config.transport == "tor" {
        transport_state
    } else {
        "ready"
    };
    NodeInfo {
        node_id: config.node_id.clone(),
        display_name: config.display_name.clone(),
        base_url: base_url.to_string(),
        healthy: transport_state == "ready",
        pools: config.pools.clone(),
        tags: config.tags.clone(),
        capabilities: config.capabilities.clone(),
        transport: config.transport.clone(),
        transport_state: transport_state.to_string(),
        transport_detail: transport_detail.unwrap_or_default().to_string(),
    }
}
