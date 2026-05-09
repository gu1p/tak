#![allow(dead_code)]

use std::fs;
use std::io::Write;
use std::net::TcpListener;
use std::path::Path;
use std::thread::{self, JoinHandle};

use prost::Message;
use tak_proto::ImageCacheStatus;

use super::remote_cli::{read_request, remote_inventory_path};

#[path = "remote_status/value.rs"]
mod value;

use value::status_value;

pub fn write_inventory(config_root: &Path, node_id: &str, base_url: &str) {
    write_inventory_entries(config_root, &[(node_id, base_url, "direct", true)]);
}

pub fn write_inventory_entries(config_root: &Path, remotes: &[(&str, &str, &str, bool)]) {
    let path = remote_inventory_path(config_root);
    fs::create_dir_all(path.parent().expect("inventory parent")).expect("create config parent");
    let mut body = String::from("version = 1\n");
    for (node_id, base_url, transport, enabled) in remotes {
        body.push_str(&format!(
            "\n[[remotes]]\nnode_id = \"{node_id}\"\ndisplay_name = \"{node_id}\"\nbase_url = \"{base_url}\"\nbearer_token = \"secret\"\npools = [\"default\"]\ntags = [\"builder\"]\ncapabilities = [\"linux\"]\ntransport = \"{transport}\"\nenabled = {enabled}\n"
        ));
    }
    fs::write(path, body).expect("write inventory");
}

pub fn status_payload(base_url: &str, with_job: bool) -> Vec<u8> {
    status_payload_for("builder-a", base_url, "direct", with_job)
}

pub fn spawn_status_server(with_job: bool) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind node status server");
    let addr = listener.local_addr().expect("listener addr");
    let base_url = format!("http://{addr}");
    let server_base_url = base_url.clone();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept status request");
        let request = read_request(&mut stream);
        assert!(
            request.starts_with("GET /v1/node/status HTTP/1.1\r\n"),
            "unexpected request: {request}"
        );
        let body = status_payload(&server_base_url, with_job);
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/x-protobuf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .expect("write response head");
        stream.write_all(&body).expect("write response body");
    });
    (base_url, server)
}

pub fn status_payload_with_image_cache(base_url: &str) -> Vec<u8> {
    let mut status = status_value("builder-a", base_url, "direct", false, "");
    status.image_cache = Some(ImageCacheStatus {
        used_bytes: 12_400_000_000,
        budget_bytes: 50_000_000_000,
        evictable_bytes: 11_000_000_000,
        entry_count: 7,
        filesystem_available_bytes: 25_000_000_000,
        filesystem_total_bytes: 100_000_000_000,
        free_floor_bytes: 10_000_000_000,
    });
    status.encode_to_vec()
}

pub fn status_payload_for(
    node_id: &str,
    base_url: &str,
    transport: &str,
    with_job: bool,
) -> Vec<u8> {
    status_payload_with_detail_for(node_id, base_url, transport, with_job, "")
}

pub fn status_payload_with_detail_for(
    node_id: &str,
    base_url: &str,
    transport: &str,
    with_job: bool,
    transport_detail: &str,
) -> Vec<u8> {
    status_value(node_id, base_url, transport, with_job, transport_detail).encode_to_vec()
}
