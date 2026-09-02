use anyhow::{Context, Result, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use tak_proto::local_daemon::v2::{Operation, Response};

use super::remote_daemon;

pub(super) async fn read_remote(node_id: &str, path: &str) -> Result<(u16, Vec<u8>)> {
    let response = remote_daemon::request(
        Operation::ReadRemote {
            node_id: node_id.to_string(),
            path: path.to_string(),
        },
        "remote-read",
    )
    .await?;
    let Response::RemoteRead {
        node_id: returned_node_id,
        http_status,
        body_base64,
        ..
    } = response
    else {
        bail!(
            "Local takd returned an unexpected remote read response; upgrade tak, takd, and workers together"
        )
    };
    if returned_node_id != node_id {
        bail!(
            "Local takd returned a mismatched remote read response; upgrade tak, takd, and workers together"
        )
    }
    let body = STANDARD
        .decode(body_base64)
        .context("decode local takd remote read payload")?;
    Ok((http_status, body))
}
