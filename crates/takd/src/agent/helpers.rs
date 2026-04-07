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
    NodeInfo {
        node_id: config.node_id.clone(),
        display_name: config.display_name.clone(),
        base_url: base_url.to_string(),
        healthy: true,
        pools: config.pools.clone(),
        tags: config.tags.clone(),
        capabilities: config.capabilities.clone(),
        transport: config.transport.clone(),
    }
}
