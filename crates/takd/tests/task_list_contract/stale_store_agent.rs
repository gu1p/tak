use std::path::Path;

use takd::agent::{InitAgentOptions, init_agent};

pub(super) fn init_direct_agent(config_root: &Path, state_root: &Path) {
    init_agent(
        config_root,
        state_root,
        InitAgentOptions {
            node_id: Some("node-a"),
            display_name: None,
            transport: Some("direct"),
            base_url: Some("http://127.0.0.1:43123"),
            pools: &[],
            tags: &[],
            capabilities: &[],
            image_cache_budget_percent: None,
            image_cache_budget_gb: None,
        },
    )
    .expect("init agent");
}
