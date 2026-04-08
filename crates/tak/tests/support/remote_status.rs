#![allow(dead_code)]

use std::fs;
use std::path::Path;

use prost::Message;
use tak_proto::{
    ActiveJob, CpuUsage, MemoryUsage, NodeInfo, NodeStatusResponse, StorageUsage, SubmittedNeed,
};

use super::remote_cli::remote_inventory_path;

pub fn write_inventory(config_root: &Path, node_id: &str, base_url: &str) {
    let path = remote_inventory_path(config_root);
    fs::create_dir_all(path.parent().expect("inventory parent")).expect("create config parent");
    fs::write(
        path,
        format!(
            "version = 1\n\n[[remotes]]\nnode_id = \"{node_id}\"\ndisplay_name = \"{node_id}\"\nbase_url = \"{base_url}\"\nbearer_token = \"secret\"\npools = [\"default\"]\ntags = [\"builder\"]\ncapabilities = [\"linux\"]\ntransport = \"direct\"\nenabled = true\n"
        ),
    )
    .expect("write inventory");
}

pub fn status_payload(base_url: &str, with_job: bool) -> Vec<u8> {
    NodeStatusResponse {
        node: Some(NodeInfo {
            node_id: "builder-a".into(),
            display_name: "builder-a".into(),
            base_url: base_url.to_string(),
            healthy: true,
            pools: vec!["default".into()],
            tags: vec!["builder".into()],
            capabilities: vec!["linux".into()],
            transport: "direct".into(),
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
