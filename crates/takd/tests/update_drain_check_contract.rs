use crate::support;

use std::process::Command;
use takd::SubmitAttemptStore;

#[test]
fn drain_check_is_offline_and_rejects_only_active_legacy_work() {
    std::fs::create_dir_all(".tmp").expect("create test temp root");
    let active = tempfile::tempdir_in(".tmp").expect("active tempdir");
    let active_state = active.path().join("state");
    let store = SubmitAttemptStore::with_db_path(active_state.join("agent.sqlite"))
        .expect("legacy submit store");
    store
        .register_submit_with_execution_root_base(
            "legacy-run",
            Some(1),
            "//:check",
            None,
            "worker-a",
            &active.path().join("exec"),
        )
        .expect("register active legacy attempt");

    let blocked = drain_check(&active_state);
    assert!(!blocked.status.success());
    assert!(
        String::from_utf8_lossy(&blocked.stderr).contains("active legacy attempts must finish")
    );

    let idle = tempfile::tempdir_in(".tmp").expect("idle tempdir");
    let allowed = drain_check(&idle.path().join("state"));
    assert!(
        allowed.status.success(),
        "drain-only check must not contact the release service: {:?}",
        allowed
    );
}

fn drain_check(state_root: &std::path::Path) -> std::process::Output {
    Command::new(support::takd_bin())
        .args(["update", "--legacy-drain-check", "--state-root"])
        .arg(state_root)
        .output()
        .expect("run takd legacy drain check")
}
