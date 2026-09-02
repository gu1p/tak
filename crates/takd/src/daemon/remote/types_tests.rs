#![cfg(test)]

use super::{NodeInfo, RemoteNodeContext, RemoteRuntimeConfig};

impl RemoteNodeContext {
    pub(in crate::daemon::remote) fn isolated_for_test() -> Self {
        Self::new(
            NodeInfo {
                node_id: "builder-a".into(),
                display_name: "builder-a".into(),
                base_url: "http://127.0.0.1:1".into(),
                healthy: true,
                pools: vec!["default".into()],
                tags: vec!["builder".into()],
                capabilities: vec!["linux".into()],
                transport: "direct".into(),
                transport_state: "ready".into(),
                transport_detail: String::new(),
            },
            "secret".into(),
            RemoteRuntimeConfig::isolated_for_test(),
        )
    }
}
