#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use super::container_runtime::simulated_container_runtime_env;
use super::live_direct::{LiveDirectRoots, init_direct_agent, spawn_direct_agent_with_env};
use super::live_direct_remote::add_remote;
use super::live_direct_token::wait_for_token;
use super::tor_smoke::{ChildGuard, takd_bin};

pub fn start_direct_agent(temp_root: &Path, workspace_root: &Path, node_id: &str) -> ChildGuard {
    let roots = LiveDirectRoots::new(temp_root);
    let takd = takd_bin();
    fs::create_dir_all(workspace_root).expect("create workspace root for direct remote helper");
    init_direct_agent(&takd, &roots, node_id);
    let serve_env = simulated_container_runtime_env(temp_root)
        .into_iter()
        .collect::<Vec<_>>();
    let agent = spawn_direct_agent_with_env(&takd, &roots, &serve_env);
    let token = wait_for_token(&takd, &roots);
    add_remote(workspace_root, &roots, &token);
    agent
}

pub fn client_env(temp_root: &Path) -> BTreeMap<String, String> {
    let roots = LiveDirectRoots::new(temp_root);
    let mut env = BTreeMap::new();
    env.insert(
        "XDG_CONFIG_HOME".to_string(),
        roots.client_config_root.display().to_string(),
    );
    env
}
