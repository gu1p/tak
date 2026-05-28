use prost::Message;
use tak_core::remote_inventory::{RemoteInventory, RemoteRecord};
use tak_proto::NodePingResponse;
use tokio::io::AsyncReadExt;

pub(super) fn inventory(remotes: Vec<RemoteRecord>) -> RemoteInventory {
    RemoteInventory {
        version: 1,
        remotes,
    }
}

pub(super) fn record(
    node_id: &str,
    transport: &str,
    enabled: bool,
    bearer_token: &str,
) -> RemoteRecord {
    RemoteRecord {
        node_id: node_id.to_string(),
        display_name: node_id.to_string(),
        base_url: format!("http://{node_id}.onion"),
        bearer_token: bearer_token.to_string(),
        pools: vec!["build".to_string()],
        tags: vec!["linux".to_string()],
        capabilities: vec!["docker".to_string()],
        transport: transport.to_string(),
        enabled,
    }
}

pub(super) fn ping() -> NodePingResponse {
    NodePingResponse {
        node_id: "builder-a".to_string(),
        protocol_version: "v1".to_string(),
        health: "healthy".to_string(),
        active_job_count: 1,
        queue_depth: 0,
        resource_summary: "cpu_available=4".to_string(),
    }
}

pub(super) fn encoded_ping_body() -> Vec<u8> {
    ping().encode_to_vec()
}

pub(super) async fn read_http_request(stream: &mut tokio::net::TcpStream) -> String {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 512];
    loop {
        let read = stream.read(&mut buffer).await.expect("read request");
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
        if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }
    String::from_utf8(bytes).expect("request utf8")
}

pub(super) fn request_contains_bearer_secret(request: &str) -> bool {
    request
        .lines()
        .any(|line| line.eq_ignore_ascii_case("Authorization: Bearer secret"))
}
