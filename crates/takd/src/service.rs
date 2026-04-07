use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use futures::StreamExt;
use safelog::DisplayRedacted;
use tokio::net::TcpListener;
use tor_cell::relaycell::msg::Connected;

use crate::agent::{
    arti_cache_dir, arti_state_dir, persist_ready_base_url, read_config, ready_context,
};
use crate::daemon::remote::{
    SubmitAttemptStore, handle_remote_v1_http_stream, run_remote_v1_http_server,
};
use crate::daemon::transport::TorHiddenServiceRuntimeConfig;

pub async fn serve_agent(config_root: &Path, state_root: &Path) -> Result<()> {
    let config = read_config(config_root)?;
    let db_path = state_root.join("agent.sqlite");
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let store =
        SubmitAttemptStore::with_db_path(db_path).context("open takd agent sqlite store")?;
    match config.transport.as_str() {
        "tor" => serve_tor_agent(config_root, state_root, store).await,
        "direct" => serve_direct_agent(config_root, state_root, &config, store).await,
        other => bail!("unsupported takd transport `{other}`"),
    }
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
    let context = ready_context(&read_config(config_root)?)?;
    run_remote_v1_http_server(listener, store, context).await
}

async fn serve_tor_agent(
    config_root: &Path,
    state_root: &Path,
    store: SubmitAttemptStore,
) -> Result<()> {
    let config = read_config(config_root)?;
    let runtime = TorHiddenServiceRuntimeConfig {
        nickname: config.hidden_service_nickname.clone(),
        state_dir: arti_state_dir(state_root),
        cache_dir: arti_cache_dir(state_root),
    };
    if let Some(bind_addr) = test_tor_hidden_service_bind_addr() {
        let base_url = format!("http://{}.onion", runtime.nickname);
        persist_ready_base_url(config_root, state_root, &base_url)?;
        let listener = TcpListener::bind(bind_addr.as_str())
            .await
            .with_context(|| format!("bind takd tor test listener at {bind_addr}"))?;
        return run_remote_v1_http_server(
            listener,
            store,
            ready_context(&read_config(config_root)?)?,
        )
        .await;
    }

    let tor_config = arti_client::config::TorClientConfigBuilder::from_directories(
        &runtime.state_dir,
        &runtime.cache_dir,
    )
    .build()
    .context("invalid Arti client configuration for takd hidden service")?;
    let tor_client = arti_client::TorClient::create_bootstrapped(tor_config)
        .await
        .context("failed to bootstrap embedded Arti for takd hidden service")?;
    let Some((running_service, rend_requests)) = tor_client
        .launch_onion_service(onion_service_config(&runtime.nickname)?)
        .context("failed to launch takd onion service via embedded Arti")?
    else {
        bail!("takd onion service launch was skipped because the service is disabled");
    };

    let base_url = running_service
        .onion_address()
        .map(|value| format!("http://{}", value.display_unredacted()))
        .ok_or_else(|| anyhow!("takd onion service did not expose an onion address"))?;
    persist_ready_base_url(config_root, state_root, &base_url)?;
    eprintln!("takd remote v1 onion service ready at {base_url}");
    let context = ready_context(&read_config(config_root)?)?;

    futures::pin_mut!(rend_requests);
    while let Some(rend_request) = rend_requests.next().await {
        let Ok(mut stream_requests) = rend_request.accept().await else {
            continue;
        };
        while let Some(stream_request) = stream_requests.next().await {
            match stream_request.accept(Connected::new_empty()).await {
                Ok(mut stream) => {
                    if let Err(err) =
                        handle_remote_v1_http_stream(&mut stream, &store, &context).await
                    {
                        eprintln!("takd onion service stream handling failed: {err}");
                    }
                }
                Err(err) => eprintln!("takd onion service stream accept failed: {err}"),
            }
        }
    }

    Ok(())
}

fn onion_service_config(
    nickname: &str,
) -> Result<arti_client::config::onion_service::OnionServiceConfig> {
    let nickname = nickname
        .trim()
        .parse()
        .with_context(|| format!("invalid tor hidden-service nickname `{nickname}`"))?;
    arti_client::config::onion_service::OnionServiceConfigBuilder::default()
        .nickname(nickname)
        .build()
        .context("invalid onion service config for takd")
}

fn test_tor_hidden_service_bind_addr() -> Option<String> {
    std::env::var("TAKD_TEST_TOR_HS_BIND_ADDR")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests;
