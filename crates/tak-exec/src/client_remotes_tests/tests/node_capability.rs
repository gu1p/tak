use tak_core::model::{RemoteSelectionSpec, RemoteSpec, RemoteTransportKind};

use crate::client_remotes::configured_remote_targets;

use super::support::{EnvVarGuard, env_lock, write_remote_inventory};

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
        selection: RemoteSelectionSpec::Sequential,
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
