use anyhow::{Result, bail};
use tak_core::model::RemoteTransportKind;

use super::super::remote_inventory::{RemoteRecord, list_remotes};
use super::DockerCliSelectors;

pub(in crate::cli::docker_cli) fn matching_remotes(
    selectors: &DockerCliSelectors,
) -> Result<Vec<RemoteRecord>> {
    let remotes = list_remotes()?
        .into_iter()
        .filter(|remote| remote.enabled)
        .filter(|remote| selector_matches_remote(selectors, remote))
        .collect::<Vec<_>>();
    Ok(remotes)
}

pub(in crate::cli::docker_cli) fn selected_transport_kind(
    transport: Option<&str>,
) -> Result<RemoteTransportKind> {
    match transport {
        None | Some("any") => Ok(RemoteTransportKind::Any),
        Some("direct") => Ok(RemoteTransportKind::Direct),
        Some("tor") => Ok(RemoteTransportKind::Tor),
        Some(other) => {
            bail!("unsupported remote transport `{other}`; expected direct, tor, or any")
        }
    }
}

fn selector_matches_remote(selectors: &DockerCliSelectors, remote: &RemoteRecord) -> bool {
    if let Some(node) = selectors.node.as_deref()
        && !node_selector_matches(node, remote)
    {
        return false;
    }
    if let Some(pool) = selectors.pool.as_deref()
        && !remote.pools.iter().any(|value| value == pool)
    {
        return false;
    }
    if let Some(transport) = selectors.transport.as_deref()
        && transport != "any"
        && remote.transport != transport
    {
        return false;
    }
    if let Some(arch) = selectors.arch.as_deref()
        && !has_capability(remote, &format!("arch:{}", normalize_arch(arch)))
    {
        return false;
    }
    if let Some(os) = selectors.os.as_deref()
        && !has_capability(remote, &format!("os:{}", normalize_os(os)))
    {
        return false;
    }
    selectors
        .tags
        .iter()
        .all(|tag| remote.tags.iter().any(|value| value == tag))
        && selectors
            .capabilities
            .iter()
            .all(|capability| has_capability(remote, capability))
}

fn node_selector_matches(selector: &str, remote: &RemoteRecord) -> bool {
    let value = selector.trim();
    !value.is_empty()
        && (remote.node_id == value
            || remote.display_name == value
            || remote.node_id.starts_with(value)
            || crate::remote_alias_for_node_id(&remote.node_id) == value)
}

fn has_capability(remote: &RemoteRecord, capability: &str) -> bool {
    remote.capabilities.iter().any(|value| {
        value == capability || normalize_capability(value) == normalize_capability(capability)
    })
}

fn normalize_capability(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

pub(in crate::cli::docker_cli) fn normalize_arch(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "aarch64" => "arm64".to_string(),
        other => other.to_string(),
    }
}

pub(in crate::cli::docker_cli) fn normalize_os(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "darwin" => "macos".to_string(),
        other => other.to_string(),
    }
}
