use tak_core::model::RemoteTransportKind;

use super::super::placement::PlacementCandidate;
use super::super::placement_remote::remote_task_candidate;
use super::support::{EnvVarGuard, remote_spec, task, write_remote_inventory};

#[test]
fn any_remote_candidate_preserves_direct_targets_without_daemon_tor_fallback() {
    let _env_lock = crate::client_remotes_tests::env_lock();
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    write_remote_inventory(
        &config_root,
        r#"
            [[remotes]]
            node_id = "builder-direct"
            base_url = "http://127.0.0.1:9000"
            bearer_token = "secret"
            pools = ["build"]
            tags = ["linux"]
            capabilities = ["docker"]
            transport = "direct"
        "#,
    );
    let _config_home = EnvVarGuard::set("XDG_CONFIG_HOME", &config_root);
    let remote = remote_spec(RemoteTransportKind::Any);

    let candidate = remote_task_candidate(&task(), &remote, None).expect("candidate");

    let PlacementCandidate::Ready(placement) = candidate else {
        panic!("expected placement candidate");
    };
    let node_ids = placement
        .ordered_remote_targets
        .iter()
        .map(|target| target.node_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(node_ids, vec!["builder-direct"]);
}

#[test]
fn any_remote_candidate_uses_daemon_tor_fallback_when_no_direct_targets_match() {
    let _env_lock = crate::client_remotes_tests::env_lock();
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    write_remote_inventory(
        &config_root,
        r#"
            [[remotes]]
            node_id = "builder-tor"
            base_url = "http://builder-tor.onion"
            bearer_token = "secret"
            pools = ["build"]
            tags = ["linux"]
            capabilities = ["docker"]
            transport = "tor"
        "#,
    );
    let _config_home = EnvVarGuard::set("XDG_CONFIG_HOME", &config_root);
    let remote = remote_spec(RemoteTransportKind::Any);

    let candidate = remote_task_candidate(&task(), &remote, None).expect("candidate");

    let PlacementCandidate::Ready(placement) = candidate else {
        panic!("expected placement candidate");
    };
    let node_ids = placement
        .ordered_remote_targets
        .iter()
        .map(|target| target.node_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(node_ids, vec!["__takd_daemon_tor__"]);
    assert!(
        placement
            .ordered_remote_targets
            .first()
            .expect("tor fallback")
            .is_daemon_tor_placement()
    );
}
