use anyhow::bail;
use tak_core::remote_inventory::RemoteRecord;
use tak_proto::worker_v2::WorkerIdentity;
use tak_proto::{NodeInfo, decode_remote_token, decode_tor_invite_payload};

use super::{RemoteAccess, RemoteAccessError};
use crate::daemon::worker_registry::WorkerConnectionTarget;

pub(super) async fn resolve(
    access: &RemoteAccess,
    invite: &str,
) -> Result<RemoteRecord, RemoteAccessError> {
    let invite = invite.trim();
    if invite.starts_with("takd:v1:") {
        return Err(RemoteAccessError::UnsupportedInvite);
    }
    if invite.starts_with("takd:v2:") {
        return resolve_direct(access, invite).await;
    }
    resolve_tor(access, invite).await
}

async fn resolve_direct(
    access: &RemoteAccess,
    invite: &str,
) -> Result<RemoteRecord, RemoteAccessError> {
    let payload = decode_remote_token(invite).map_err(|_| RemoteAccessError::ProtocolMismatch)?;
    let node = payload.node.ok_or_else(|| {
        RemoteAccessError::Failed(anyhow::anyhow!(
            "direct v2 invite is missing worker identity"
        ))
    })?;
    ensure_direct_invite(&node, &payload.bearer_token).map_err(RemoteAccessError::Failed)?;
    let target = WorkerConnectionTarget {
        node_id: node.node_id.clone(),
        endpoint: node.base_url.clone(),
        bearer_token: payload.bearer_token.clone(),
        transport: "direct".into(),
    };
    let identity = probe_identity(access, &target).await?;
    ensure_direct_matches(&node, &identity).map_err(RemoteAccessError::Failed)?;
    Ok(remote_record(
        identity,
        node.base_url,
        payload.bearer_token,
        "direct",
    ))
}

async fn resolve_tor(
    access: &RemoteAccess,
    invite: &str,
) -> Result<RemoteRecord, RemoteAccessError> {
    let payload = decode_tor_invite_payload(invite).map_err(RemoteAccessError::Failed)?;
    let target = WorkerConnectionTarget {
        node_id: "remote-onboarding".into(),
        endpoint: payload.base_url.clone(),
        bearer_token: payload.bearer_token.clone(),
        transport: "tor".into(),
    };
    let identity = probe_identity(access, &target).await?;
    ensure_tor_matches(&payload.base_url, &identity).map_err(RemoteAccessError::Failed)?;
    Ok(remote_record(
        identity,
        payload.base_url,
        payload.bearer_token,
        "tor",
    ))
}

async fn probe_identity(
    access: &RemoteAccess,
    target: &WorkerConnectionTarget,
) -> Result<WorkerIdentity, RemoteAccessError> {
    let response = access
        .broker
        .worker_v2_http_exchange(target, "GET", "/v2/worker/identity", &[])
        .await
        .map_err(RemoteAccessError::Failed)?;
    if response.status == 426 {
        return Err(RemoteAccessError::ProtocolMismatch);
    }
    if response.status != 200 {
        return Err(RemoteAccessError::Failed(anyhow::anyhow!(
            "remote onboarding failed with HTTP {}",
            response.status
        )));
    }
    tak_proto::worker_v2::decode_identity(&response.body)
        .map_err(|_| RemoteAccessError::ProtocolMismatch)
}

fn remote_record(
    identity: WorkerIdentity,
    base_url: String,
    bearer_token: String,
    transport: &str,
) -> RemoteRecord {
    RemoteRecord {
        node_id: identity.node_id,
        display_name: identity.display_name,
        base_url,
        bearer_token,
        pools: identity.pools,
        tags: identity.tags,
        capabilities: identity.capabilities,
        transport: transport.into(),
        enabled: true,
    }
}

fn ensure_direct_invite(node: &NodeInfo, bearer_token: &str) -> anyhow::Result<()> {
    let (host, _) = tak_core::endpoint::endpoint_host_port(&node.base_url)?;
    if node.node_id.trim().is_empty() || node.transport != "direct" || host.ends_with(".onion") {
        bail!("direct v2 invite identity is invalid");
    }
    if bearer_token.trim().is_empty() || bearer_token.chars().any(char::is_control) {
        bail!("direct v2 invite bearer token is invalid");
    }
    Ok(())
}

fn ensure_direct_matches(invited: &NodeInfo, identity: &WorkerIdentity) -> anyhow::Result<()> {
    if identity.node_id != invited.node_id
        || identity.base_url != invited.base_url
        || identity.transport != "direct"
    {
        bail!("direct invite does not match worker v2 identity");
    }
    Ok(())
}

fn ensure_tor_matches(invited_base_url: &str, identity: &WorkerIdentity) -> anyhow::Result<()> {
    if identity.transport != "tor" || identity.base_url != invited_base_url {
        bail!(
            "tor invite expected {invited_base_url} via tor, worker identity returned {} via {}",
            identity.base_url,
            identity.transport
        );
    }
    Ok(())
}
