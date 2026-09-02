#![allow(dead_code)] // shared by test binaries that each exercise a subset of helpers

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use anyhow::Result;
use tak_core::model::WorkspaceSpec;

use super::local_daemon::LocalDaemonGuard;

pub fn install_fake_make(root: &Path, script: &str) -> Result<String> {
    let bin = root.join("bin");
    fs::create_dir_all(&bin)?;
    let fake_make = bin.join("make");
    fs::write(&fake_make, script)?;
    fs::set_permissions(&fake_make, fs::Permissions::from_mode(0o755))?;

    let mut paths = vec![bin];
    paths.extend(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    ));
    Ok(std::env::join_paths(paths)?.to_string_lossy().into_owned())
}

pub fn start_local_daemon(
    workspace: &Path,
    environment: &mut BTreeMap<String, String>,
) -> LocalDaemonGuard {
    let socket = workspace.with_extension("make-takd.sock");
    environment.insert("TAKD_SOCKET".into(), socket.display().to_string());
    LocalDaemonGuard::spawn_with_tor_dial_addr(
        &socket,
        &empty_spec(workspace),
        "127.0.0.1:9".into(),
    )
}

pub fn start_inventory_daemon(
    workspace: &Path,
    environment: &mut BTreeMap<String, String>,
) -> LocalDaemonGuard {
    let socket = workspace.with_extension("make-takd.sock");
    let inventory = Path::new(
        environment
            .get("XDG_CONFIG_HOME")
            .expect("remote Make test config root"),
    )
    .join("tak/remotes.toml");
    environment.insert("TAKD_SOCKET".into(), socket.display().to_string());
    LocalDaemonGuard::spawn_with_tor_inventory(
        &socket,
        &empty_spec(workspace),
        "127.0.0.1:9".into(),
        inventory,
    )
}

pub fn start_container_daemon(
    workspace: &Path,
    environment: &mut BTreeMap<String, String>,
    runtime_path: &str,
) -> Result<LocalDaemonGuard> {
    let socket = workspace.with_extension("make-takd.sock");
    let wrapper = workspace.with_extension("make-takd-attempt");
    let takd = super::takd_binary::takd_bin();
    fs::write(
        &wrapper,
        format!(
            "#!/bin/sh\nexport PATH='{runtime_path}'\nexport TAK_TEST_HOST_PLATFORM=other\n\
             export TAK_TEST_IGNORE_HOST_USAGE=1\nexport TAKD_MEMORY_PRESSURE_ENABLED=false\n\
             exec '{}' \"$@\"\n",
            takd.display()
        ),
    )?;
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755))?;
    environment.insert("TAKD_SOCKET".into(), socket.display().to_string());
    Ok(LocalDaemonGuard::spawn_with_attempt_executable(
        &socket,
        &empty_spec(workspace),
        wrapper,
    ))
}

fn empty_spec(root: &Path) -> WorkspaceSpec {
    WorkspaceSpec {
        project_id: "make-v2".into(),
        root: root.to_path_buf(),
        tasks: BTreeMap::new(),
        sessions: BTreeMap::new(),
        limiters: HashMap::new(),
        queues: HashMap::new(),
    }
}
