use anyhow::{Context, Result, anyhow, bail};
use futures::StreamExt;
use safelog::DisplayRedacted;
use std::path::Path;
use tokio::net::TcpListener;
use tor_cell::relaycell::msg::Connected;

use crate::agent::{
    arti_cache_dir, arti_state_dir, persist_ready_base_url, read_config, ready_context,
};
use crate::daemon::remote::{
    SubmitAttemptStore, handle_remote_v1_http_stream, run_remote_v1_http_server,
};
use crate::daemon::transport::TorHiddenServiceRuntimeConfig;

mod probe;
mod startup_policy;

use startup_policy::startup_probe_retry_policy;

pub(super) async fn serve_tor_agent(
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
        return serve_test_bind(
            config_root,
            state_root,
            &config,
            &runtime.nickname,
            store,
            &bind_addr,
        )
        .await;
    }

    tracing::info!(
        "bootstrapping embedded Arti for takd hidden service nickname {}",
        runtime.nickname
    );
    let tor_client = arti_client::TorClient::create_bootstrapped(tor_client_config(&runtime)?)
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
    let context = ready_context(&ready_config(&config, &base_url))?;
    crate::daemon::remote::spawn_remote_cleanup_janitor(context.shared_status_state());
    let readiness_client = tor_client.isolated_client();
    let startup_probe_retry_policy = startup_probe_retry_policy();
    let readiness = probe::wait_for_tor_hidden_service_startup(
        &readiness_client,
        &base_url,
        &config.bearer_token,
        startup_probe_retry_policy.timeout,
        startup_probe_retry_policy.backoff,
    );
    tokio::pin!(readiness);
    futures::pin_mut!(rend_requests);
    let spawn_rend_request =
        |rend_request: tor_hsservice::RendRequest,
         store: SubmitAttemptStore,
         context: crate::daemon::remote::RemoteNodeContext| {
            std::mem::drop(tokio::spawn(async move {
                let accepted = rend_request.accept().await;
                let mut stream_requests = match accepted {
                    Ok(stream_requests) => stream_requests,
                    Err(err) => {
                        tracing::error!("takd onion service rendezvous accept failed: {err}");
                        return;
                    }
                };
                while let Some(stream_request) = stream_requests.next().await {
                    match stream_request.accept(Connected::new_empty()).await {
                        Ok(mut stream) => {
                            let store = store.clone();
                            let context = context.clone();
                            std::mem::drop(tokio::spawn(async move {
                                if let Err(err) =
                                    handle_remote_v1_http_stream(&mut stream, &store, &context)
                                        .await
                                {
                                    tracing::error!(
                                        "takd onion service stream handling failed: {err}"
                                    );
                                }
                            }));
                        }
                        Err(err) => {
                            tracing::error!("takd onion service stream accept failed: {err}")
                        }
                    }
                }
            }));
        };

    loop {
        tokio::select! {
            ready = &mut readiness => {
                ready?;
                persist_ready_base_url(config_root, state_root, &base_url)?;
                tracing::info!("takd remote v1 onion service ready at {base_url}");
                break;
            }
            maybe_rend_request = rend_requests.next() => {
                let Some(rend_request) = maybe_rend_request else {
                    bail!("takd onion service stopped before readiness probe completed");
                };
                spawn_rend_request(rend_request, store.clone(), context.clone());
            }
        }
    }

    while let Some(rend_request) = rend_requests.next().await {
        spawn_rend_request(rend_request, store.clone(), context.clone());
    }
    Ok(())
}

pub(super) fn onion_service_config(
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

pub(super) fn test_tor_hidden_service_bind_addr() -> Option<String> {
    std::env::var("TAKD_TEST_TOR_HS_BIND_ADDR")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn ready_config(config: &crate::agent::AgentConfig, base_url: &str) -> crate::agent::AgentConfig {
    let mut ready = config.clone();
    ready.base_url = Some(base_url.to_string());
    ready
}

fn tor_client_config(
    runtime: &TorHiddenServiceRuntimeConfig,
) -> Result<arti_client::config::TorClientConfig> {
    arti_client::config::TorClientConfigBuilder::from_directories(
        &runtime.state_dir,
        &runtime.cache_dir,
    )
    .build()
    .context("invalid Arti client configuration for takd hidden service")
}

async fn serve_test_bind(
    config_root: &Path,
    state_root: &Path,
    config: &crate::agent::AgentConfig,
    nickname: &str,
    store: SubmitAttemptStore,
    bind_addr: &str,
) -> Result<()> {
    let base_url = format!("http://{nickname}.onion");
    tracing::info!("using takd tor hidden-service test bind override at {bind_addr}");
    let listener = TcpListener::bind(bind_addr)
        .await
        .with_context(|| format!("bind takd tor test listener at {bind_addr}"))?;
    persist_ready_base_url(config_root, state_root, &base_url)?;
    tracing::info!("takd remote v1 onion service ready at {base_url}");
    run_remote_v1_http_server(
        listener,
        store,
        ready_context(&ready_config(config, &base_url))?,
    )
    .await
}

#[cfg(test)]
mod tests;
