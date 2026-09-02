#[cfg(unix)]
#[test]
fn chmod_einval_fallback_accepts_docker_mode_without_non_owner_write_access() {
    assert!(super::socket_permissions::mode_allows_owner_connection(
        0o755
    ));
    assert!(!super::socket_permissions::mode_allows_owner_connection(
        0o775
    ));
    assert!(!super::socket_permissions::mode_allows_owner_connection(
        0o757
    ));
    assert!(!super::socket_permissions::mode_allows_owner_connection(
        0o555
    ));
}

#[cfg(unix)]
#[test]
fn chmod_einval_fallback_anchors_host_owned_socket_to_its_parent() {
    let container_euid = 0;
    let bind_mount_uid = 501;
    assert_ne!(container_euid, bind_mount_uid);
    assert!(super::socket_permissions::fallback_access_is_owner_only(
        true,
        bind_mount_uid,
        0o755,
        true,
        bind_mount_uid,
        0o755,
    ));
    assert!(super::socket_permissions::fallback_access_is_owner_only(
        true,
        bind_mount_uid,
        0o666,
        true,
        bind_mount_uid,
        0o700,
    ));
}

#[cfg(unix)]
#[test]
fn chmod_einval_fallback_rejects_unprotected_socket_or_parent() {
    use super::socket_permissions::fallback_access_is_owner_only as secure;

    assert!(!secure(true, 502, 0o755, true, 501, 0o755));
    assert!(!secure(true, 501, 0o775, true, 501, 0o755));
    assert!(!secure(true, 501, 0o755, true, 501, 0o775));
    assert!(!secure(true, 501, 0o666, true, 501, 0o755));
    assert!(!secure(false, 501, 0o755, true, 501, 0o755));
    assert!(!secure(true, 501, 0o755, false, 501, 0o755));
}

#[cfg(unix)]
#[test]
fn chmod_einval_fallback_rejects_each_traversable_socket_write_class() {
    use super::socket_permissions::fallback_access_is_owner_only as secure;

    assert!(secure(true, 501, 0o620, true, 501, 0o700));
    assert!(!secure(true, 501, 0o620, true, 501, 0o710));
    assert!(!secure(true, 501, 0o620, true, 501, 0o701));
    assert!(secure(true, 501, 0o602, true, 501, 0o700));
    assert!(!secure(true, 501, 0o602, true, 501, 0o701));
    assert!(!secure(true, 501, 0o602, true, 501, 0o710));
}

#[cfg(unix)]
#[test]
fn chmod_einval_failure_reports_the_exact_access_tuple() {
    let message = super::socket_permissions::fallback_access_diagnostic(
        false, 501, 0o140755, false, 502, 0o040700, 0,
    );

    assert_eq!(
        message,
        "socket directory does not protect owner-only access: \
socket_is_socket=false socket_uid=501 socket_mode=0o140755 \
parent_is_dir=false parent_uid=502 parent_mode=0o40700 euid=0"
    );
}
