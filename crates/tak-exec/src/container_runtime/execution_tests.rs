use super::execution::diagnostics::exit_137_diagnostic_message;
use tak_core::model::ContainerMountSpec;

#[test]
fn exit_137_diagnostic_uses_engine_evidence_without_inventing_a_cause() {
    let confirmed = exit_137_diagnostic_message(Some(true), None);
    assert!(confirmed.contains("OOMKilled=true"));
    assert!(confirmed.contains("container OOM confirmed"));

    let disproved = exit_137_diagnostic_message(Some(false), None);
    assert!(disproved.contains("OOMKilled=false"));
    assert!(disproved.contains("cause is unknown"));
    assert!(!disproved.contains("host-level SIGKILL"));
    assert!(!disproved.contains("kernel OOM"));
    assert!(!disproved.contains("systemd-oomd"));

    let unavailable = exit_137_diagnostic_message(None, None);
    assert!(unavailable.contains("OOMKilled=unknown"));
    assert!(unavailable.contains("cause is unknown"));
}

#[cfg(unix)]
#[test]
fn authored_mounts_resolve_inside_the_workspace_and_reject_symlink_escapes() {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    let outside = temp.path().join("outside");
    std::fs::create_dir_all(workspace.join("cache")).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    let safe = ContainerMountSpec {
        source: "cache".into(),
        target: "/var/cache/build".into(),
        read_only: true,
    };
    let binds = super::execution::authored_mount_binds(&workspace, &[safe]).unwrap();
    assert_eq!(binds.len(), 1);
    assert!(binds[0].ends_with(":/var/cache/build:ro"), "{binds:?}");

    symlink(&outside, workspace.join("escape")).unwrap();
    let escaping = ContainerMountSpec {
        source: "escape".into(),
        target: "/mnt".into(),
        read_only: false,
    };
    let error = super::execution::authored_mount_binds(&workspace, &[escaping])
        .unwrap_err()
        .to_string();
    assert!(error.contains("symlink escape"), "{error}");
    assert!(error.contains("daemon-owned workspace"), "{error}");
}
