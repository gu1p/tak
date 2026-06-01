//! Contract for repo-root task context declarations.

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::Result;
use tak_core::label::parse_label;
use tak_core::model::{IgnoreSourceSpec, PathAnchor};
use tak_loader::{LoadOptions, load_workspace};

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
}

#[test]
fn repo_root_check_task_opts_into_gitignore_context() -> Result<()> {
    let spec = load_workspace(repo_root(), &LoadOptions::default())?;
    let label = parse_label("//:check", "//").expect("check label");
    let task = spec.tasks.get(&label).expect("check task");

    assert!(
        task.context
            .ignored
            .iter()
            .any(|source| matches!(source, IgnoreSourceSpec::GitIgnore)),
        "expected //:check to declare gitignore() in its context"
    );
    Ok(())
}

#[test]
fn repo_root_check_context_keeps_cataloged_remote_example_fixtures() -> Result<()> {
    let spec = load_workspace(repo_root(), &LoadOptions::default())?;
    let label = parse_label("//:check", "//").expect("check label");
    let task = spec.tasks.get(&label).expect("check task");
    let includes: BTreeSet<_> = task
        .context
        .include
        .iter()
        .filter(|path| path.anchor == PathAnchor::Workspace)
        .map(|path| path.path.as_str())
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
