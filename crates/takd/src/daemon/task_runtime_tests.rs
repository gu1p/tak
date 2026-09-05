#[cfg(unix)]
#[test]
fn daemon_container_user_matches_the_attempt_workspace_owner() {
    // SAFETY: these calls have no preconditions and only read process credentials.
    let (uid, gid) = unsafe { (libc::geteuid(), libc::getegid()) };

    assert_eq!(
        super::task_runtime::daemon_container_user().as_deref(),
        Some(format!("{uid}:{gid}").as_str())
    );
}
