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
    let (direct_agent, tor_agent, client_config_root, fixture_env) =
        remote_fixture(entry, temp_root, workspace_root)?;
    if let Some(config_root) = client_config_root {
        env.insert(
            "XDG_CONFIG_HOME".to_string(),
            config_root.to_string_lossy().into_owned(),
        );
    }
    env.extend(fixture_env);
    let local_daemon = daemon_guard(entry, temp_root, &spec, &mut env);
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
    let needs_tor_broker = entry.remote_fixture.as_deref() == Some("tor_onion_http");
    if !entry.requires_daemon && !needs_tor_broker {
        return None;
    }
    let socket_path = temp_root.join("takd.sock");
    env.insert(
        "TAKD_SOCKET".to_string(),
        socket_path.to_string_lossy().into_owned(),
    );
    Some(
        match (
            env.get("TAK_TEST_TOR_ONION_DIAL_ADDR"),
            env.get("XDG_CONFIG_HOME"),
        ) {
            (Some(dial_addr), Some(config_root)) => LocalDaemonGuard::spawn_with_tor_inventory(
                &socket_path,
                spec,
                dial_addr.clone(),
                Path::new(config_root).join("tak/remotes.toml"),
            ),
            (Some(dial_addr), None) => {
                LocalDaemonGuard::spawn_with_tor_dial_addr(&socket_path, spec, dial_addr.clone())
            }
            (None, _) => LocalDaemonGuard::spawn(&socket_path, spec),
        },
    )
}
