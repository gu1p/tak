use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use tokio::net::TcpListener;

use crate::agent::{persist_ready_base_url, read_config};
use crate::daemon::remote::{SubmitAttemptStore, run_remote_v1_http_server};

mod tor;

pub async fn serve_agent(config_root: &Path, state_root: &Path) -> Result<()> {
    let config = read_config(config_root)?;
    tracing::info!("starting takd serve for transport {}", config.transport);
    let db_path = state_root.join("agent.sqlite");
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let store =
        SubmitAttemptStore::with_db_path(db_path).context("open takd agent sqlite store")?;
    match config.transport.as_str() {
        "tor" => tor::serve_tor_agent(config_root, state_root, store).await,
        "direct" => serve_direct_agent(config_root, state_root, &config, store).await,
        other => bail!("unsupported takd transport `{other}`"),
    }
}

#[doc(hidden)]
pub fn observe_live_tor_client_stream(context: &crate::daemon::remote::RemoteNodeContext) {
    tor::handle_accepted_stream_side_effects(context);
}

async fn serve_direct_agent(
    config_root: &Path,
    state_root: &Path,
    config: &crate::agent::AgentConfig,
    store: SubmitAttemptStore,
) -> Result<()> {
    let configured_base_url = config
        .base_url
        .as_deref()
        .ok_or_else(|| anyhow!("base_url must be http(s) when serving direct transport"))?;
    let (scheme, bind_addr) = configured_base_url
        .strip_prefix("http://")
        .map(|value| ("http", value))
        .or_else(|| {
            configured_base_url
                .strip_prefix("https://")
                .map(|value| ("https", value))
        })
        .ok_or_else(|| anyhow!("base_url must be http(s) when serving direct transport"))?;
    let listener = TcpListener::bind(bind_addr)
        .await
        .with_context(|| format!("bind takd http listener at {bind_addr}"))?;
    let advertised_base_url = if bind_addr.ends_with(":0") {
        format!("{scheme}://{}", listener.local_addr()?)
    } else {
        configured_base_url.to_string()
    };
    persist_ready_base_url(config_root, state_root, &advertised_base_url)?;
    tracing::info!("takd remote v1 direct service ready at {advertised_base_url}");
    let context = crate::agent::ready_context(&read_config(config_root)?)?;
    run_remote_v1_http_server(listener, store, context).await
}

#[cfg(test)]
mod tests;
