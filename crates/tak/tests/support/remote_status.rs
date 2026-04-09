#![allow(dead_code)]

use std::fs;
use std::path::Path;

use prost::Message;
use tak_proto::{
    ActiveJob, CpuUsage, MemoryUsage, NodeInfo, NodeStatusResponse, StorageUsage, SubmittedNeed,
};

use super::remote_cli::remote_inventory_path;

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

pub fn status_payload_for(
    node_id: &str,
    base_url: &str,
    transport: &str,
    with_job: bool,
) -> Vec<u8> {
    NodeStatusResponse {
        node: Some(NodeInfo {
            node_id: node_id.into(),
            display_name: node_id.into(),
            base_url: base_url.to_string(),
            healthy: true,
            pools: vec!["default".into()],
            tags: vec!["builder".into()],
            capabilities: vec!["linux".into()],
            transport: transport.into(),
        }),
        sampled_at_ms: 1_734_000_000_000,
        cpu: Some(CpuUsage {
            utilization_percent: Some(12.5),
            logical_cores: 8,
        }),
        memory: Some(MemoryUsage {
            used_bytes: 2_048,
            total_bytes: 8_192,
        }),
        storage: Some(StorageUsage {
            path: "/tmp/takd-remote-exec".into(),
            total_bytes: 10_000,
            available_bytes: 7_000,
            used_bytes: 3_000,
            tak_execution_bytes: 256,
        }),
        allocated_needs: vec![],
        active_jobs: if with_job {
            vec![ActiveJob {
                task_run_id: "task-run-1".into(),
                attempt: 1,
                task_label: "//apps/web:build".into(),
                started_at_ms: 1_734_000_000_000,
                needs: vec![SubmittedNeed {
                    name: "cpu".into(),
                    scope: "machine".into(),
                    scope_key: None,
                    slots: 2.0,
                }],
                execution_root_bytes: 256,
                runtime: Some("containerized".into()),
            }]
        } else {
            vec![]
        },
    }
    .encode_to_vec()
}
