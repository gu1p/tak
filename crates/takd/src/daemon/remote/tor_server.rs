use super::*;

pub async fn run_remote_v1_tor_hidden_service(
    config: TorHiddenServiceRuntimeConfig,
    store: SubmitAttemptStore,
) -> Result<()> {
    if let Some(test_bind_addr) = test_tor_hidden_service_bind_addr() {
        let listener = TcpListener::bind(test_bind_addr.as_str())
            .await
            .with_context(|| {
                format!("failed to bind takd tor test listener at {test_bind_addr}")
            })?;
        return run_remote_v1_http_server(listener, store).await;
    }

    let tor_client_config = arti_client::config::TorClientConfigBuilder::from_directories(
        &config.state_dir,
        &config.cache_dir,
    )
    .build()
    .context("invalid Arti client configuration for takd hidden service")?;
    let tor_client = arti_client::TorClient::create_bootstrapped(tor_client_config)
        .await
        .context("failed to bootstrap embedded Arti for takd hidden service")?;
    let onion_service_config = build_tor_hidden_service_config(&config.nickname)?;
    let Some((running_service, rend_requests)) = tor_client
        .launch_onion_service(onion_service_config)
        .context("failed to launch takd onion service via embedded Arti")?
    else {
        bail!("takd onion service launch was skipped because the service is disabled");
    };

    let onion_endpoint = running_service
        .onion_address()
        .map(|hsid| format!("http://{}", hsid.display_unredacted()))
        .ok_or_else(|| anyhow!("takd onion service did not expose an onion address"))?;
    eprintln!("takd remote v1 onion service ready at {onion_endpoint}");

    futures::pin_mut!(rend_requests);
    while let Some(rend_request) = rend_requests.next().await {
        let accepted = rend_request.accept().await;
        let mut stream_requests = match accepted {
            Ok(stream_requests) => stream_requests,
            Err(err) => {
                eprintln!("takd onion service rendezvous accept failed: {err}");
                continue;
            }
        };

        while let Some(stream_request) = stream_requests.next().await {
            match stream_request.accept(Connected::new_empty()).await {
                Ok(mut stream) => {
                    if let Err(err) = handle_remote_v1_http_stream(&mut stream, &store).await {
                        eprintln!("takd onion service stream handling failed: {err}");
                    }
                }
                Err(err) => {
                    eprintln!("takd onion service stream accept failed: {err}");
                }
            }
        }
    }

    Ok(())
}
fn build_tor_hidden_service_config(
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

pub(crate) fn remote_v1_bind_addr_from_env() -> Option<String> {
    std::env::var("TAKD_REMOTE_V1_BIND_ADDR")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(crate) fn tor_hidden_service_runtime_config_from_env()
-> Result<Option<TorHiddenServiceRuntimeConfig>> {
    let nickname = match std::env::var("TAKD_TOR_HS_NICKNAME") {
        Ok(value) => value.trim().to_string(),
        Err(_) => return Ok(None),
    };
    if nickname.is_empty() {
        return Ok(None);
    }

    let state_dir = std::env::var("TAKD_TOR_STATE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir().join("takd-arti-state"));
    let cache_dir = std::env::var("TAKD_TOR_CACHE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir().join("takd-arti-cache"));

    Ok(Some(TorHiddenServiceRuntimeConfig {
        nickname,
        state_dir,
        cache_dir,
    }))
}

pub(super) fn test_tor_hidden_service_bind_addr() -> Option<String> {
    std::env::var("TAKD_TEST_TOR_HS_BIND_ADDR")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
