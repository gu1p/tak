use takd::{RemoteNodeContext, RemoteRuntimeConfig};

pub fn test_context() -> RemoteNodeContext {
    test_context_with_runtime(super::runtime_config::isolated())
}

pub fn test_context_with_runtime(runtime_config: RemoteRuntimeConfig) -> RemoteNodeContext {
    test_context_for_node_with_runtime("builder-a", runtime_config)
}

pub fn test_context_for_node_with_runtime(
    node_id: &str,
    runtime_config: RemoteRuntimeConfig,
) -> RemoteNodeContext {
    RemoteNodeContext::new(
        tak_proto::NodeInfo {
            node_id: node_id.into(),
            display_name: node_id.into(),
            base_url: "http://127.0.0.1:43123".into(),
            healthy: true,
            pools: vec!["default".into()],
            tags: vec!["builder".into()],
            capabilities: vec!["linux".into()],
            transport: "direct".into(),
            transport_state: "ready".into(),
            transport_detail: String::new(),
        },
        "secret".into(),
        runtime_config,
    )
}
