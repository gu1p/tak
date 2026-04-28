#![cfg(test)]

use crate::agent::{AgentConfig, AgentImageCacheConfig};

use super::pending_context;

#[test]
fn pending_context_rejects_invalid_configured_image_cache() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config = AgentConfig {
        node_id: "builder".to_string(),
        display_name: "builder".to_string(),
        base_url: None,
        bearer_token: "secret".to_string(),
        pools: vec!["default".to_string()],
        tags: vec!["builder".to_string()],
        capabilities: vec!["linux".to_string()],
        transport: "tor".to_string(),
        hidden_service_nickname: "builder".to_string(),
        image_cache: Some(AgentImageCacheConfig {
            budget_gb: 0.0,
            mutable_tag_ttl_secs: 86_400,
            sweep_interval_secs: 60,
            low_disk_min_free_percent: 10.0,
            low_disk_min_free_gb: 10.0,
        }),
    };

    let err = match pending_context(&config, "http://builder.onion", temp.path()) {
        Ok(_) => panic!("invalid image cache config should fail"),
        Err(err) => err,
    };

    assert!(
        err.to_string()
            .contains("image cache budget must be a positive number"),
        "unexpected error: {err:#}"
    );
}
