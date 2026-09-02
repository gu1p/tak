use anyhow::{Result, bail};
use tak_proto::local_daemon::v2::{Operation, Response};

use super::remote_daemon;

pub(in crate::cli) use tak_proto::local_daemon::v2::RemoteInventoryEntry as RemoteRecord;

pub(super) async fn preview_remote(invite: &str) -> Result<RemoteRecord> {
    let response = remote_daemon::request(
        Operation::PreviewRemote {
            invite: invite.to_string(),
        },
        "remote-preview",
    )
    .await?;
    let Response::RemotePreview { remote, .. } = response else {
        bail!(
            "Local takd returned an unexpected remote preview response; upgrade tak, takd, and workers together"
        )
    };
    Ok(remote)
}

pub(super) async fn add_remote(invite: &str) -> Result<RemoteRecord> {
    let response = remote_daemon::request(
        Operation::AddRemote {
            invite: invite.to_string(),
        },
        "remote-add",
    )
    .await?;
    let Response::RemoteAdded { remote, .. } = response else {
        bail!(
            "Local takd returned an unexpected remote add response; upgrade tak, takd, and workers together"
        )
    };
    Ok(remote)
}

pub(super) async fn list_remotes() -> Result<Vec<RemoteRecord>> {
    let response = remote_daemon::request(Operation::ListRemotes {}, "remote-list").await?;
    let Response::RemoteList { remotes, .. } = response else {
        bail!(
            "Local takd returned an unexpected remote list response; upgrade tak, takd, and workers together"
        )
    };
    Ok(remotes)
}

pub(super) async fn remove_remote(node_id: &str) -> Result<bool> {
    let response = remote_daemon::request(
        Operation::RemoveRemote {
            node_id: node_id.to_string(),
        },
        "remote-remove",
    )
    .await?;
    let Response::RemoteRemoved {
        node_id: returned_node_id,
        removed,
        ..
    } = response
    else {
        bail!(
            "Local takd returned an unexpected remote removal response; upgrade tak, takd, and workers together"
        )
    };
    if returned_node_id != node_id {
        bail!(
            "Local takd returned a mismatched remote removal response; upgrade tak, takd, and workers together"
        )
    }
    Ok(removed)
}
