use std::io::{Write, stdout};

use anyhow::{Result, bail};

use super::remote_read::read_remote;

pub(super) async fn run_remote_logs(node_id: &str, all: bool, lines: usize) -> Result<()> {
    let node_id = required_node_id(node_id)?;
    let path = if all {
        "/v2/worker/logs?all=true".to_string()
    } else {
        format!("/v2/worker/logs?lines={lines}")
    };
    let (status, body) = read_remote(node_id, &path).await?;
    if status != 200 {
        bail!("remote node {node_id} logs failed with HTTP {status}");
    }
    stdout().write_all(&body)?;
    Ok(())
}

pub(super) fn required_node_id(node_id: &str) -> Result<&str> {
    let node_id = node_id.trim();
    if node_id.is_empty() {
        bail!("--node is required");
    }
    Ok(node_id)
}
