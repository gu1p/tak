#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use tak_loader::{LoadOptions, load_workspace};

use super::examples_catalog::ExampleEntry;
use super::examples_remote_fixture::remote_fixture;
use super::local_daemon::LocalDaemonGuard;
use super::tor_probe_env::insert_live_tor_probe_env;
use super::tor_smoke::ChildGuard;

pub struct ExampleRunContext {
    pub env: BTreeMap<String, String>,
    _local_daemon: Option<LocalDaemonGuard>,
    _direct_agent: Option<ChildGuard>,
    _tor_agent: Option<ChildGuard>,
}

pub fn setup_example_run(
    entry: &ExampleEntry,
    temp_root: &Path,
    workspace_root: &Path,
) -> Result<ExampleRunContext> {
    let spec = load_workspace(workspace_root, &LoadOptions::default())
        .with_context(|| format!("load staged workspace for {}", entry.name))?;
    let mut env = BTreeMap::new();
    let local_daemon = daemon_guard(entry, temp_root, &spec, &mut env);
    let (direct_agent, tor_agent, client_config_root) =
        remote_fixture(entry, temp_root, workspace_root)?;
    if let Some(config_root) = client_config_root {
        env.insert(
            "XDG_CONFIG_HOME".to_string(),
            config_root.to_string_lossy().into_owned(),
        );
    }
    if entry.remote_fixture.as_deref() == Some("tor_onion_http") {
        insert_live_tor_probe_env(&mut env);
    }
    Ok(ExampleRunContext {
        env,
        _local_daemon: local_daemon,
        _direct_agent: direct_agent,
        _tor_agent: tor_agent,
    })
}

fn daemon_guard(
    entry: &ExampleEntry,
    temp_root: &Path,
    spec: &tak_core::model::WorkspaceSpec,
    env: &mut BTreeMap<String, String>,
) -> Option<LocalDaemonGuard> {
    if !entry.requires_daemon {
        return None;
    }
    let socket_path = temp_root.join("takd.sock");
    env.insert(
        "TAKD_SOCKET".to_string(),
        socket_path.to_string_lossy().into_owned(),
    );
    Some(LocalDaemonGuard::spawn(&socket_path, spec))
}
