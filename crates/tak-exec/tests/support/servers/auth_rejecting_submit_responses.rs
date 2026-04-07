#![allow(dead_code)]

use tak_proto::{ErrorResponse, NodeInfo};

pub fn shutdown() -> ErrorResponse {
    ErrorResponse {
        message: "shutdown".into(),
    }
}

pub fn auth_failed() -> ErrorResponse {
    ErrorResponse {
        message: "auth_failed".into(),
    }
}

pub fn not_found(path: &str) -> ErrorResponse {
    ErrorResponse {
        message: format!("unexpected:{path}"),
    }
}

pub fn node_info(node_id: &str, port: u16) -> NodeInfo {
    NodeInfo {
        node_id: node_id.into(),
        display_name: node_id.into(),
        base_url: format!("http://127.0.0.1:{port}"),
        healthy: true,
        pools: vec!["build".into()],
        tags: vec!["builder".into()],
        capabilities: vec!["linux".into()],
        transport: "direct".into(),
    }
}
