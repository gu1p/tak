#![allow(dead_code)]

mod endpoint;
mod response;

use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
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
    let config = arti_client::config::TorClientConfigBuilder::from_directories(
        root.join("client-state"),
        root.join("client-cache"),
    )
    .build()
    .context("build test Arti config")?;
    let client = arti_client::TorClient::create_bootstrapped(config)
        .await
        .context("bootstrap separate test Arti client")?;
    let (host, port) = endpoint_host_port(base_url)?;
    let authority = endpoint_socket_addr(base_url)?;
    let deadline = Instant::now() + Duration::from_secs(120);

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
