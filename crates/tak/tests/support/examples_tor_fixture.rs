#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;

use super::container_runtime::simulated_container_runtime_env;
use super::examples_catalog::ExampleEntry;
use super::examples_remote_fixture::{RemoteFixtureSetup, expected_remote_node};
use super::live_tor::{
    LiveTorRoots, init_tor_agent, spawn_tor_agent, spawn_tor_agent_with_env,
    wait_for_token as wait_for_tor_token,
};
use super::live_tor_remote::{
    add_remote as add_tor_remote, add_remote_with_env as add_tor_remote_with_env,
};
use super::tor_smoke::takd_bin;

pub fn tor_fixture(
    entry: &ExampleEntry,
    temp_root: &Path,
    workspace_root: &Path,
) -> Result<RemoteFixtureSetup> {
    let takd = takd_bin();
    let roots = LiveTorRoots::new(temp_root);
    init_tor_agent(
        &takd,
        &roots,
        expected_remote_node(entry, "remote-tor-default"),
    );
    let mut client_env = BTreeMap::new();
    let mut serve_env = if entry.simulate_container_runtime {
        simulated_container_runtime_env(temp_root)
            .into_iter()
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let bind_addr = reserve_local_bind_addr()?;
    serve_env.push(("TAKD_TEST_TOR_HS_BIND_ADDR".into(), bind_addr.clone()));
    client_env.insert("TAK_TEST_TOR_ONION_DIAL_ADDR".into(), bind_addr);
    let agent = spawn_tor_agent_with_env(&takd, &roots, &serve_env);
    let token = wait_for_tor_token(&takd, &roots);
    if client_env.is_empty() {
        add_tor_remote(workspace_root, &roots, &token);
    } else {
        add_tor_remote_with_env(workspace_root, &roots, &token, &client_env);
    }
    Ok((
        None,
        Some(agent),
        Some(roots.client_config_root),
        client_env,
    ))
}

fn reserve_local_bind_addr() -> Result<String> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?.to_string();
    drop(listener);
    Ok(addr)
}
