use anyhow::{Result, bail};
use std::io::{Write, stdout};

use super::remote_http::get_remote_bytes;
use super::remote_inventory::{RemoteRecord, list_remotes};

pub(super) async fn run_remote_logs(node_id: &str, all: bool, lines: usize) -> Result<()> {
    let remote = selected_remote(node_id)?;
    let path = if all {
        "/v1/node/logs?all=true".to_string()
    } else {
        format!("/v1/node/logs?lines={lines}")
    };
    let (status, body) = get_remote_bytes(&remote, &path).await?;
    if status != 200 {
        bail!(
            "remote node {} logs failed with HTTP {status}",
            remote.node_id
        );
    }
    stdout().write_all(&body)?;
    Ok(())
}

pub(super) fn selected_remote(node_id: &str) -> Result<RemoteRecord> {
    let wanted = node_id.trim();
    if wanted.is_empty() {
        bail!("--node is required");
    }
    list_remotes()?
        .into_iter()
        .find(|remote| remote.enabled && remote.node_id == wanted)
        .ok_or_else(|| anyhow::anyhow!("enabled remote not found: {wanted}"))
}
