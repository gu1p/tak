use tak_core::model::RemoteTransportKind;

use super::super::placement::PlacementCandidate;
use super::super::placement_remote::remote_task_candidate;
use super::support::{EnvVarGuard, remote_spec, task, write_remote_inventory};

#[test]
fn tor_remote_candidate_uses_daemon_placeholder_without_client_inventory() {
    let _env_lock = crate::client_remotes_tests::env_lock();
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    write_remote_inventory(&config_root, "not valid toml");
    let _config_home = EnvVarGuard::set("XDG_CONFIG_HOME", &config_root);
    let remote = remote_spec(RemoteTransportKind::Tor);

    let candidate = remote_task_candidate(&task(), &remote, None).expect("candidate");

    let PlacementCandidate::Ready(placement) = candidate else {
        panic!("expected daemon placement candidate");
    };
    let target = placement
        .strict_remote_target
        .as_ref()
        .expect("daemon placeholder target");
    assert!(target.is_daemon_tor_placement());
    assert!(placement.ordered_remote_targets.is_empty());
    assert_eq!(target.required_pool.as_deref(), Some("build"));
    assert_eq!(target.required_tags, vec!["linux"]);
    assert_eq!(target.required_capabilities, vec!["docker"]);
}
