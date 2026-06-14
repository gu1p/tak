#![cfg(test)]
use tak_core::model::{SessionReuseSpec, SessionUseSpec};

use super::ExecutionSessionManager;

fn share_workspace_session(name: &str, display_name: &str) -> SessionUseSpec {
    SessionUseSpec {
        name: name.to_string(),
        display_name: display_name.to_string(),
        execution: None,
        reuse: SessionReuseSpec::ShareWorkspace,
        context: None,
    }
}

#[test]
fn prepared_session_copies_identity_and_keys_by_run_id() {
    let manager = ExecutionSessionManager::new("run-7".to_string());
    let session = share_workspace_session("build", "Build");

    let prepared = manager.prepared_session(&session, None, None);

    assert_eq!(prepared.key, "run-7-build");
    assert_eq!(prepared.name, "build");
    assert_eq!(prepared.display_name, "Build");
    assert!(matches!(prepared.reuse, SessionReuseSpec::ShareWorkspace));
    assert!(prepared.root.is_none());
}
