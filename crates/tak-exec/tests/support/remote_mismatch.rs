#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use crate::support::RemoteInventoryRecord;
use crate::support::{EnvGuard, write_remote_inventory};

pub fn prepare_workspace(env: &mut EnvGuard) -> (tempfile::TempDir, PathBuf, PathBuf) {
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp.path().join("workspace");
    let config_root = temp.path().join("config");
    fs::create_dir_all(&workspace_root).expect("create workspace");
    env.set("XDG_CONFIG_HOME", config_root.display().to_string());
    (temp, workspace_root, config_root)
}

pub fn write_enabled_remote_mismatches(config_root: &Path) {
    write_remote_inventory(
        config_root,
        &[
            RemoteInventoryRecord {
                node_id: "builder-default".into(),
                base_url: "http://builder-default.onion".into(),
                bearer_token: "secret".into(),
                pools: vec!["default".into()],
                tags: vec!["builder".into()],
                capabilities: vec!["linux".into()],
                transport: "tor".into(),
                enabled: true,
            },
            RemoteInventoryRecord {
                node_id: "builder-direct".into(),
                base_url: "http://builder-direct".into(),
                bearer_token: "secret".into(),
                pools: vec!["build".into()],
                tags: vec!["builder".into()],
                capabilities: vec!["linux".into()],
                transport: "direct".into(),
                enabled: true,
            },
            RemoteInventoryRecord {
                node_id: "builder-macos".into(),
                base_url: "http://builder-macos.onion".into(),
                bearer_token: "secret".into(),
                pools: vec!["build".into()],
                tags: vec!["runner".into()],
                capabilities: vec!["macos".into()],
                transport: "tor".into(),
                enabled: true,
            },
        ],
    );
}

pub fn write_disabled_remote(config_root: &Path) {
    write_remote_inventory(
        config_root,
        &[RemoteInventoryRecord {
            node_id: "builder-disabled".into(),
            base_url: "http://builder-disabled.onion".into(),
            bearer_token: "secret".into(),
            pools: vec!["build".into()],
            tags: vec!["builder".into()],
            capabilities: vec!["linux".into()],
            transport: "tor".into(),
            enabled: false,
        }],
    );
}
