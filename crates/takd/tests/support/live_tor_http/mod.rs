#![allow(dead_code)]

mod endpoint;
mod response;
mod timeout;

use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail};
use tak_proto::NodeInfo;
use tokio::time::{sleep, timeout};

use self::endpoint::{endpoint_host_port, endpoint_socket_addr};
use self::response::fetch_node_info;
pub use self::timeout::{format_live_tor_wait_timeout, live_tor_test_timeout};

pub async fn wait_for_onion_node_info(root: &Path, base_url: &str, bearer_token: &str) -> NodeInfo {
    wait_for_node_info_result(root, base_url, bearer_token)
        .await
        .expect("wait for onion node info")
}

async fn wait_for_node_info_result(
    root: &Path,
    base_url: &str,
    bearer_token: &str,
) -> Result<NodeInfo> {
    let deadline = Instant::now() + live_tor_test_timeout();
    let client = bootstrap_client(root, deadline)
        .await
        .context("bootstrap separate test Arti client")?;
    let (host, port) = endpoint_host_port(base_url)?;
    let authority = endpoint_socket_addr(base_url)?;

    loop {
        let last_error = match timeout(
            Duration::from_secs(15),
            client.connect((host.as_str(), port)),
        )
        .await
        {
            Ok(Ok(stream)) => {
                match fetch_node_info(stream, &authority, bearer_token, base_url).await {
                    Ok(node) => return Ok(node),
                    Err(err) => err.context("fetch onion node info from separate test Arti client"),
                }
            }
            Ok(Err(err)) => {
                anyhow!(err).context("connect separate test Arti client to onion service")
            }
            Err(_) => anyhow!(
                "connect separate test Arti client to onion service timed out after 15000ms"
            ),
        };
        if Instant::now() >= deadline {
            bail!(
                "{}",
                format_live_tor_wait_timeout(base_url, Some(&last_error))
            );
        }
        sleep(Duration::from_secs(1)).await;
    }
}

async fn bootstrap_client(
    root: &Path,
    deadline: Instant,
) -> Result<arti_client::TorClient<tor_rtcompat::PreferredRuntime>> {
    let mut last_error = anyhow!("timed out bootstrapping separate test Arti client");
    loop {
        let config = arti_client::config::TorClientConfigBuilder::from_directories(
            root.join("client-state"),
            root.join("client-cache"),
        )
        .build()
        .context("build test Arti config")?;
        match timeout(
            Duration::from_secs(60),
            arti_client::TorClient::create_bootstrapped(config),
        )
        .await
        {
            Ok(Ok(client)) => return Ok(client),
            Ok(Err(error)) => last_error = error.into(),
            Err(_) => {}
        }
        if Instant::now() >= deadline {
            return Err(last_error);
        }
        sleep(Duration::from_secs(1)).await;
    }
}
