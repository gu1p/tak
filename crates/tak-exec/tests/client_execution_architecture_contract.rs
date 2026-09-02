use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn tak_exec_does_not_register_or_export_the_retired_client_engine() -> Result<()> {
    let library = read(&crate_root().join("src/lib.rs"))?;
    for retired in [
        "mod client_remotes;",
        "mod engine;",
        "mod execution_graph;",
        "mod lease_client;",
        "mod remote_protocol_codec;",
        "mod retry;",
        "mod task_run_metadata;",
        "run_resolved_task",
        "run_tasks",
        "RunOptions",
        "RunSummary",
        "TaskRunResult",
    ] {
        assert!(
            !library.contains(retired),
            "tak-exec must not expose or register retired client execution API `{retired}`"
        );
    }
    Ok(())
}

#[test]
fn tak_exec_has_no_client_preferred_node_upload_selection() -> Result<()> {
    assert_source_tree_excludes(
        &crate_root().join("src"),
        &["preferred_node_id", "x-tak-preferred-node"],
    )
}

#[test]
fn tak_cli_does_not_import_the_retired_client_execution_path() -> Result<()> {
    assert_source_tree_excludes(
        &crate_root().join("../tak/src"),
        &[
            "tak_exec::run_tasks",
            "tak_exec::run_resolved_task",
            "RemoteTargetSelection",
            "StrictRemoteTarget",
            "configured_remote_targets",
            "tak_exec::workspace_upload",
            "preferred_node_id",
            "x-tak-preferred-node",
            "evaluate_named_policy_decision",
        ],
    )
}

#[test]
fn retained_worker_api_remains_compile_checked() {
    let _cancel = tak_exec::RunCancellation::new();
    let _execute = tak_exec::execute_remote_worker_steps_with_cancellation;
    let _execute_with_output = tak_exec::execute_remote_worker_steps_with_output_and_cancellation;
    let _cache_status = tak_exec::image_cache_status;
    let _cache_janitor = tak_exec::run_image_cache_janitor_once;
}

fn assert_source_tree_excludes(root: &Path, forbidden: &[&str]) -> Result<()> {
    for entry in fs::read_dir(root).with_context(|| format!("read {}", root.display()))? {
        let path = entry?.path();
        if path.is_dir() {
            assert_source_tree_excludes(&path, forbidden)?;
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
            let source = read(&path)?;
            for token in forbidden {
                assert!(
                    !source.contains(token),
                    "{} retains retired client execution token `{token}`",
                    path.display()
                );
            }
        }
    }
    Ok(())
}

fn read(path: &Path) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("read {}", path.display()))
}
