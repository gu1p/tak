#![allow(dead_code)]

mod endpoint;
mod response;

use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail};
use tak_proto::NodeInfo;
use tokio::time::{sleep, timeout};

use self::endpoint::{endpoint_host_port, endpoint_socket_addr};
use self::response::fetch_node_info;

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
    let deadline = Instant::now() + Duration::from_secs(180);
    let client = bootstrap_client(root, deadline)
        .await
        .context("bootstrap separate test Arti client")?;
    let (host, port) = endpoint_host_port(base_url)?;
    let authority = endpoint_socket_addr(base_url)?;

    loop {
        if let Ok(Ok(mut stream)) = timeout(
            Duration::from_secs(15),
            client.connect((host.as_str(), port)),
        )
        .await
            && let Ok(node) = fetch_node_info(&mut stream, &authority, bearer_token, base_url).await
        {
            return Ok(node);
        }
        if Instant::now() >= deadline {
            bail!("timed out waiting for separate Arti client to reach {base_url}");
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
