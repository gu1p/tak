use tak_core::model::{RemoteSelectionSpec, RemoteSpec, RemoteTransportKind};

use crate::{
    client_remotes::configured_remote_targets, engine::remote_models::StrictRemoteTransportKind,
};

use super::support::{EnvVarGuard, env_lock, write_remote_inventory};

#[test]
fn any_transport_request_builds_only_concrete_direct_targets() {
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
        selection: RemoteSelectionSpec::Sequential,
        session: None,
    })
    .expect("selection should succeed");

    let transports = selection
        .matched_targets
        .iter()
        .map(|target| target.transport_kind)
        .collect::<Vec<_>>();
    let expected = vec![StrictRemoteTransportKind::Direct];
    assert_eq!(transports, expected);
}
