use takd::agent::{AgentConfig, AutoUpdateConfig, UpdateNetwork};

const MINIMAL: &str = r#"
node_id = "n1"
display_name = "n1"
bearer_token = "tok"
pools = []
tags = []
capabilities = []
transport = "tor"
hidden_service_nickname = "nick"
"#;

#[test]
fn agent_config_without_auto_update_table_uses_defaults() {
    let config: AgentConfig = toml::from_str(MINIMAL).unwrap();
    let update = config.auto_update;
    assert!(update.enabled);
    assert!(update.auto_apply);
    assert!(update.require_signature);
    assert_eq!(update.check_interval_hours, 24);
    assert_eq!(update.jitter_hours, 6);
    assert_eq!(update.drain_timeout_secs, 1800);
    assert!(update.include_sibling_tak);
    assert!(!update.allow_downgrade);
    assert!(update.network.is_none());
}

#[test]
fn network_resolves_from_transport_when_unset() {
    let update = AutoUpdateConfig::default();
    assert_eq!(update.effective_network("direct"), UpdateNetwork::Clearnet);
    assert_eq!(update.effective_network("tor"), UpdateNetwork::Disabled);
    assert!(update.loop_enabled("direct"));
    assert!(!update.loop_enabled("tor"));
}

#[test]
fn explicit_network_overrides_transport_default() {
    let update = AutoUpdateConfig {
        network: Some(UpdateNetwork::Clearnet),
        ..Default::default()
    };
    assert_eq!(update.effective_network("tor"), UpdateNetwork::Clearnet);
    assert!(update.loop_enabled("tor"));
}

#[test]
fn disabled_flag_stops_the_loop() {
    let update = AutoUpdateConfig {
        enabled: false,
        ..Default::default()
    };
    assert!(!update.loop_enabled("direct"));
}

#[test]
fn parses_explicit_lowercase_network_table() {
    let raw = r#"
node_id = "n1"
display_name = "n1"
bearer_token = "tok"
pools = []
tags = []
capabilities = []
transport = "tor"
hidden_service_nickname = "nick"

[auto_update]
network = "clearnet"
auto_apply = false
"#;
    let config: AgentConfig = toml::from_str(raw).unwrap();
    assert_eq!(config.auto_update.network, Some(UpdateNetwork::Clearnet));
    assert!(!config.auto_update.auto_apply);
    assert!(config.auto_update.enabled);
}
