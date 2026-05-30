use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use tokio::net::TcpListener;

use crate::agent::{
    DirectBaseUrlError, parse_direct_base_url, persist_ready_base_url, read_config,
};
use crate::daemon::peer_manager::PeerManager;
use crate::daemon::protocol::TorBroker;
use crate::daemon::remote::{SubmitAttemptStore, run_remote_v1_http_server};
use crate::daemon::runtime::{default_socket_path, run_local_daemon_with_broker_and_peers};

mod control;
mod tor;

use control::{AgentControlState, spawn_agent_control_socket};

pub async fn serve_agent(config_root: &Path, state_root: &Path) -> Result<()> {
    let config_result = read_config(config_root);
    let broker = broker_for_transport(state_root, config_result.as_ref().ok());
    spawn_local_daemon_socket(state_root, broker.clone());
    let config = match config_result {
        Ok(config) => config,
        Err(_err) if !config_root.join("agent.toml").exists() => {
            tracing::info!("starting takd serve as local daemon only; agent.toml not found");
            std::future::pending::<()>().await;
            unreachable!("pending future returned");
        }
        Err(err) => return Err(err),
    };
    tracing::info!("starting takd serve for transport {}", config.transport);
    let db_path = state_root.join("agent.sqlite");
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let store =
        SubmitAttemptStore::with_db_path(db_path).context("open takd agent sqlite store")?;
    abandon_unfinished_submits(&store)?;
    let control_state = AgentControlState::default();
    spawn_agent_control_socket(state_root, store.clone(), control_state.clone())?;
    match config.transport.as_str() {
        "tor" => tor::serve_tor_agent(config_root, state_root, store, control_state, broker).await,
        "direct" => {
            serve_direct_agent(config_root, state_root, &config, store, control_state).await
        }
        other => bail!("unsupported takd transport `{other}`"),
    }
}

// On `tor` transport the broker borrows the hidden-service Tor client (one Arti
// client serves the onion and dials peers); otherwise it bootstraps its own.
fn broker_for_transport(
    state_root: &Path,
    config: Option<&crate::agent::AgentConfig>,
) -> TorBroker {
    if config.is_some_and(|config| config.transport == "tor") {
        TorBroker::for_shared_tor_client(state_root.to_path_buf())
    } else {
        TorBroker::for_state_root(state_root.to_path_buf())
    }
}

fn spawn_local_daemon_socket(state_root: &Path, broker: TorBroker) {
    let socket_path = default_socket_path();
    let db_path = state_root.join("takd.sqlite");
    if let Some(parent) = db_path.parent()
        && let Err(err) = std::fs::create_dir_all(parent)
    {
        tracing::error!("failed to create local daemon state directory: {err:#}");
        return;
    }
    let peers = if let Ok(path) = tak_core::remote_inventory::default_remote_inventory_path() {
        let peers = tak_core::remote_inventory::load_remote_inventory_at(&path)
            .map(PeerManager::from_inventory)
            .unwrap_or_else(|err| {
                tracing::warn!("starting local daemon with empty remote inventory: {err:#}");
                PeerManager::default()
            });
        peers.spawn_inventory_reloader_with_broker(path, broker.clone());
        peers
    } else {
        PeerManager::default()
    };
    peers.spawn_heartbeat_loop(broker.clone());
    tokio::spawn(async move {
        if let Err(err) =
            run_local_daemon_with_broker_and_peers(&socket_path, &db_path, broker, peers).await
        {
            tracing::error!("takd local daemon socket failed: {err:#}");
        }
    });
}

fn abandon_unfinished_submits(store: &SubmitAttemptStore) -> Result<()> {
    let abandoned = store.mark_unfinished_attempts_abandoned()?;
    if abandoned > 0 {
        tracing::warn!("marked {abandoned} unfinished takd task attempt(s) as abandoned");
    }
    Ok(())
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
    control_state: AgentControlState,
) -> Result<()> {
    let parsed = parse_direct_base_url(config.base_url.as_deref()).map_err(|err| match err {
        DirectBaseUrlError::Missing | DirectBaseUrlError::InvalidScheme => {
            anyhow!("base_url must be http(s) when serving direct transport")
        }
        DirectBaseUrlError::MissingHost => anyhow!("base_url must include a host"),
        DirectBaseUrlError::MissingPort => anyhow!("base_url must include a port"),
        DirectBaseUrlError::UnsupportedComponents => {
            anyhow!("base_url must not include userinfo, path, query, or fragment when serving direct transport")
        }
    })?;
    let bind_addr = parsed.bind_addr();
    let listener = TcpListener::bind(&bind_addr)
        .await
        .with_context(|| format!("bind takd http listener at {bind_addr}"))?;
    let advertised_base_url = if bind_addr.ends_with(":0") {
        format!("{}://{}", parsed.scheme, listener.local_addr()?)
    } else {
        parsed.canonical_base_url()
    };
    persist_ready_base_url(config_root, state_root, &advertised_base_url)?;
    tracing::info!("takd remote v1 direct service ready at {advertised_base_url}");
    let context =
        crate::agent::ready_context_with_state_root(&read_config(config_root)?, state_root)?;
    control_state.set_context(context.clone())?;
    run_remote_v1_http_server(listener, store, context).await
}

#[cfg(test)]
mod tests;
