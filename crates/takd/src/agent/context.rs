use std::path::Path;

use anyhow::{Result, anyhow};

use super::AgentConfig;
use super::helpers::node_info;
use crate::daemon::remote::{RemoteNodeContext, RemoteRuntimeConfig};

/// Build a ready remote-node context for `config` (requires an advertised base URL).
///
/// ```no_run
/// # // Reason: needs a fully constructed AgentConfig and reads runtime config from the environment.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub fn ready_context(config: &AgentConfig) -> Result<RemoteNodeContext> {
    let base_url = config
        .base_url
        .clone()
        .ok_or_else(|| anyhow!("agent token not ready"))?;
    Ok(RemoteNodeContext::new(
        node_info(config, &base_url),
        config.bearer_token.clone(),
        RemoteRuntimeConfig::from_env(),
    ))
}

/// Like [`ready_context`], also wiring the state root and image-cache config.
///
/// ```no_run
/// # // Reason: needs a constructed AgentConfig plus a filesystem state root and reads runtime config from the environment.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub fn ready_context_with_state_root(
    config: &AgentConfig,
    state_root: &Path,
) -> Result<RemoteNodeContext> {
    let base_url = config
        .base_url
        .clone()
        .ok_or_else(|| anyhow!("agent token not ready"))?;
    let mut context = RemoteNodeContext::new(
        node_info(config, &base_url),
        config.bearer_token.clone(),
        RemoteRuntimeConfig::from_env(),
    )
    .with_state_root(state_root);
    if let Some(image_cache) = &config.image_cache {
        context = context.with_image_cache_config(image_cache.runtime_config(state_root)?);
    }
    Ok(context)
}
