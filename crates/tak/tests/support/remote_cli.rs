#![allow(dead_code)]

use std::io::Read;
use std::path::{Path, PathBuf};

use tak_proto::{NodeInfo, RemoteTokenPayload, encode_remote_token};

pub fn node_info(node_id: &str, base_url: &str, transport: &str) -> NodeInfo {
    NodeInfo {
        node_id: node_id.into(),
        display_name: node_id.into(),
        base_url: base_url.into(),
        healthy: true,
        pools: vec!["default".into()],
        tags: vec!["builder".into()],
        capabilities: vec!["linux".into()],
        transport: transport.into(),
        transport_state: "ready".into(),
        transport_detail: String::new(),
    }
}

pub fn remote_token(node_id: &str, base_url: &str, transport: &str) -> String {
    encode_remote_token(&RemoteTokenPayload {
        version: "v1".into(),
        node: Some(node_info(node_id, base_url, transport)),
        bearer_token: "secret".into(),
    })
    .expect("encode remote token")
}

pub fn read_request(stream: &mut impl Read) -> String {
    let mut request = Vec::new();
    let mut buf = [0_u8; 256];
    loop {
        let read = stream.read(&mut buf).expect("read request");
        if read == 0 {
            break;
        }
        request.extend_from_slice(&buf[..read]);
        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }
    String::from_utf8(request).expect("request utf8")
}

pub fn remote_inventory_path(config_root: &Path) -> PathBuf {
    config_root.join("tak").join("remotes.toml")
}
