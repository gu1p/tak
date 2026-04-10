#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;

use super::container_runtime::simulated_container_runtime_env;
use super::examples_catalog::ExampleEntry;
use super::examples_remote_fixture::{RemoteFixtureSetup, expected_remote_node};
use super::live_direct::{
    LiveDirectRoots, init_direct_agent, spawn_direct_agent, spawn_direct_agent_with_env,
};
use super::live_direct_remote::add_remote as add_direct_remote;
use super::live_direct_token::wait_for_token;
use super::tor_smoke::takd_bin;

pub fn direct_fixture(
    entry: &ExampleEntry,
    temp_root: &Path,
    workspace_root: &Path,
) -> Result<RemoteFixtureSetup> {
    let takd = takd_bin();
    let roots = LiveDirectRoots::new(temp_root);
    init_direct_agent(
        &takd,
        &roots,
        expected_remote_node(entry, "remote-direct-default"),
    );
    let serve_env = if entry.simulate_container_runtime {
        simulated_container_runtime_env(temp_root)
            .into_iter()
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let agent = if serve_env.is_empty() {
        spawn_direct_agent(&takd, &roots)
    } else {
        spawn_direct_agent_with_env(&takd, &roots, &serve_env)
    };
    let token = wait_for_token(&takd, &roots);
    add_direct_remote(workspace_root, &roots, &token);
    Ok((
        Some(agent),
        None,
        Some(roots.client_config_root),
        BTreeMap::new(),
    ))
}
