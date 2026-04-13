#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::Path;

use super::live_direct::{LiveDirectRoots, init_direct_agent, spawn_direct_agent};
use super::live_direct_remote::add_remote;
use super::live_direct_token::wait_for_token;
use super::tor_smoke::{ChildGuard, takd_bin};

pub fn attach_direct_remote(workspace_root: &Path, roots: &LiveDirectRoots) -> ChildGuard {
    let takd = takd_bin();
    init_direct_agent(&takd, roots, "declared-output-builder");
    let agent = spawn_direct_agent(&takd, roots);
    let token = wait_for_token(&takd, roots);
    add_remote(workspace_root, roots, &token);
    agent
}

pub fn remote_env(roots: &LiveDirectRoots) -> BTreeMap<String, String> {
    let mut env = BTreeMap::new();
    env.insert(
        "XDG_CONFIG_HOME".to_string(),
        roots.client_config_root.display().to_string(),
    );
    env
}
