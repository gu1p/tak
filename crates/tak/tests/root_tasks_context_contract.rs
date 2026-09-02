//! Contract for repo-root task context declarations.

use std::collections::BTreeSet;

use anyhow::Result;

use crate::support::root_task_contracts::{load_root_module, task};

#[test]
fn repo_root_check_task_opts_into_gitignore_context() -> Result<()> {
    let module = load_root_module()?;
    let context = task(&module, "//:check")
        .context
        .as_ref()
        .expect("check context");

    assert!(
        context.use_gitignore,
        "expected //:check to declare gitignore() in its context"
    );
    Ok(())
}

#[test]
fn repo_root_check_context_keeps_cataloged_remote_example_fixtures() -> Result<()> {
    let module = load_root_module()?;
    let includes: BTreeSet<_> = task(&module, "//:check")
        .context
        .as_ref()
        .expect("check context")
        .include
        .iter()
        .map(String::as_str)
        .collect();

    for path in [
        "examples/large/27_hybrid_local_remote_test_suite_success/TASKS.py",
        "examples/large/27_hybrid_local_remote_test_suite_success/apps/web/TASKS.py",
        "examples/large/28_hybrid_local_remote_test_suite_failure_with_logs/TASKS.py",
        "examples/large/28_hybrid_local_remote_test_suite_failure_with_logs/apps/web/TASKS.py",
        "examples/large/29_remote_any_transport_container_log_storm/TASKS.py",
        "examples/large/29_remote_any_transport_container_log_storm/apps/logstorm/TASKS.py",
        "examples/large/30_remote_session_share_paths/TASKS.py",
        "examples/large/31_remote_session_share_workspace/TASKS.py",
    ] {
        assert!(
            includes.contains(path),
            "expected //:check context to explicitly include {path}"
        );
    }

    Ok(())
}
