#![cfg(test)]

use crate::agent::{AgentConfig, AgentImageCacheConfig};

use super::{pending_context, recovering_exit};

#[test]
fn pending_context_rejects_invalid_configured_image_cache() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut config = valid_config();
    config.image_cache = Some(AgentImageCacheConfig {
        budget_gb: 0.0,
        mutable_tag_ttl_secs: 86_400,
        sweep_interval_secs: 60,
        low_disk_min_free_percent: 10.0,
        low_disk_min_free_gb: 10.0,
    });

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

#[test]
fn recovery_exit_marks_the_context_before_returning() {
    let temp = tempfile::tempdir().expect("tempdir");
    let context = pending_context(&valid_config(), "http://builder.onion", temp.path())
        .expect("pending context");
    context.mark_transport_ready().expect("mark ready");

    let exit = recovering_exit(
        &context,
        temp.path(),
        "http://builder.onion",
        "rend stream ended",
    )
    .expect("recovery exit");

    assert_eq!(exit.reason, "rend stream ended");
    let node = context.node_info().expect("recovering node");
    assert_eq!(node.transport_state, "recovering");
    assert_eq!(node.transport_detail, "rend stream ended");
}

fn valid_config() -> AgentConfig {
    AgentConfig {
        node_id: "builder".to_string(),
        display_name: "builder".to_string(),
        base_url: None,
        bearer_token: "secret".to_string(),
        pools: vec!["default".to_string()],
        tags: vec!["builder".to_string()],
        capabilities: vec!["linux".to_string()],
        transport: "tor".to_string(),
        hidden_service_nickname: "builder".to_string(),
        image_cache: None,
        auto_update: Default::default(),
    }
}
