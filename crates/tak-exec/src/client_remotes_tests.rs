#![cfg(test)]

use std::fs;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use tak_core::model::{RemoteSpec, RemoteTransportKind};

use crate::{
    client_remotes::configured_remote_targets, engine::remote_models::StrictRemoteTransportKind,
};

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn any_transport_request_builds_only_concrete_strict_targets() {
    let _env_lock = env_lock();
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    write_remote_inventory(
        &config_root,
        r#"
            [[remotes]]
            node_id = "builder-direct"
            base_url = "http://127.0.0.1:8080"
            bearer_token = "secret"
            pools = ["build"]
            tags = ["builder"]
            capabilities = ["linux"]
            transport = "direct"
            enabled = true

            [[remotes]]
            node_id = "builder-tor"
            base_url = "http://builder-tor.onion"
            bearer_token = "secret"
            pools = ["build"]
            tags = ["builder"]
            capabilities = ["linux"]
            transport = "tor"
            enabled = true
        "#,
    );
    let _config_home = EnvVarGuard::set("XDG_CONFIG_HOME", &config_root);

    let selection = configured_remote_targets(&RemoteSpec {
        pool: Some("build".into()),
        required_tags: vec!["builder".into()],
        required_capabilities: vec!["linux".into()],
        transport_kind: RemoteTransportKind::Any,
        runtime: None,
        selection: tak_core::model::RemoteSelectionSpec::Sequential,
        session: None,
    })
    .expect("selection should succeed");

    let transports = selection
        .matched_targets
        .iter()
        .map(|target| target.transport_kind)
        .collect::<Vec<_>>();
    let expected = vec![
        StrictRemoteTransportKind::Direct,
        StrictRemoteTransportKind::Tor,
    ];
    assert_eq!(transports, expected);
}

#[test]
fn node_capability_matches_candidate_node_id_without_inventory_capability() {
    let _env_lock = env_lock();
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    write_remote_inventory(
        &config_root,
        r#"
            [[remotes]]
            node_id = "builder-direct"
            base_url = "http://127.0.0.1:8080"
            bearer_token = "secret"
            pools = ["build"]
            tags = ["builder"]
            capabilities = ["linux"]
            transport = "direct"
            enabled = true

            [[remotes]]
            node_id = "builder-other"
            base_url = "http://127.0.0.1:8081"
            bearer_token = "secret"
            pools = ["build"]
            tags = ["builder"]
            capabilities = ["linux"]
            transport = "direct"
            enabled = true
        "#,
    );
    let _config_home = EnvVarGuard::set("XDG_CONFIG_HOME", &config_root);

    let selection = configured_remote_targets(&RemoteSpec {
        pool: Some("build".into()),
        required_tags: vec!["builder".into()],
        required_capabilities: vec!["node:builder-direct".into()],
        transport_kind: RemoteTransportKind::Direct,
        runtime: None,
        selection: tak_core::model::RemoteSelectionSpec::Sequential,
        session: None,
    })
    .expect("selection should succeed");

    let matched_node_ids = selection
        .matched_targets
        .iter()
        .map(|target| target.node_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(matched_node_ids, vec!["builder-direct"]);
}

#[rustfmt::skip]
fn env_lock() -> MutexGuard<'static, ()> { ENV_LOCK.lock().expect("env lock") }

fn write_remote_inventory(config_root: &Path, content: &str) {
    let tak_dir = config_root.join("tak");
    fs::create_dir_all(&tak_dir).expect("create config dir");
    fs::write(tak_dir.join("remotes.toml"), content).expect("write inventory");
}

struct EnvVarGuard {
    key: &'static str,
    original: Option<String>,
}

#[rustfmt::skip]
impl EnvVarGuard {
    fn set(key: &'static str, value: &Path) -> Self { let original = std::env::var(key).ok(); unsafe { std::env::set_var(key, value); } Self { key, original } }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match self.original.as_ref() {
            Some(value) => unsafe {
                std::env::set_var(self.key, value);
            },
            None => unsafe {
                std::env::remove_var(self.key);
            },
        }
    }
}
