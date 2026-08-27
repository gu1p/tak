use std::path::Path;

use anyhow::Result;

use crate::agent::{
    TransportHealth, TransportState, node_info_with_transport, write_transport_health,
};
use crate::daemon::remote::{RemoteNodeContext, RemoteRuntimeConfig};

use super::TorSessionExit;

pub(super) fn pending_context(
    config: &crate::agent::AgentConfig,
    base_url: &str,
    state_root: &Path,
) -> Result<RemoteNodeContext> {
    let ready = super::ready_config(config, base_url);
    let mut context = RemoteNodeContext::new(
        node_info_with_transport(&ready, base_url, TransportState::Pending.as_str(), None),
        config.bearer_token.clone(),
        RemoteRuntimeConfig::from_env(),
    );
    if let Some(image_cache) = &config.image_cache {
        context = context.with_image_cache_config(image_cache.runtime_config(state_root)?);
    }
    Ok(context)
}

pub(super) fn mark_transport_ready(
    context: &RemoteNodeContext,
    state_root: &Path,
    base_url: &str,
) -> Result<()> {
    context.mark_transport_ready()?;
    write_transport_health(
        state_root,
        &TransportHealth::ready(Some(base_url.to_string())),
    )
}

pub(super) fn mark_transport_pending(
    context: &RemoteNodeContext,
    state_root: &Path,
    base_url: &str,
    detail: impl Into<String>,
) -> Result<()> {
    let detail = detail.into();
    context.set_transport_state(TransportState::Pending.as_str(), Some(&detail))?;
    write_transport_health(
        state_root,
        &TransportHealth::new(
            TransportState::Pending,
            Some(base_url.to_string()),
            Some(detail),
        ),
    )
}

pub(super) fn mark_transport_recovering(
    context: &RemoteNodeContext,
    state_root: &Path,
    base_url: &str,
    detail: impl Into<String>,
) -> Result<()> {
    let detail = detail.into();
    context.set_transport_state(TransportState::Recovering.as_str(), Some(&detail))?;
    write_transport_health(
        state_root,
        &TransportHealth::recovering(Some(base_url.to_string()), Some(detail)),
    )
}

pub(super) fn recovering_exit(
    context: &RemoteNodeContext,
    state_root: &Path,
    base_url: &str,
    reason: impl Into<String>,
) -> Result<TorSessionExit> {
    let reason = reason.into();
    mark_transport_recovering(context, state_root, base_url, reason.clone())?;
    Ok(TorSessionExit {
        base_url: base_url.to_string(),
        reason,
    })
}

#[path = "live_state_tests.rs"]
mod live_state_tests;
