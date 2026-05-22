use std::path::{Path, PathBuf};

use super::support::remote_workspace;
use crate::support::EnvGuard;
use tak_core::model::TaskLabel;
use tak_exec::{RunOptions, run_tasks};

pub(super) fn configure_workspace(env: &mut EnvGuard) -> (tempfile::TempDir, PathBuf, PathBuf) {
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace = temp.path().join("workspace");
    let config = temp.path().join("config");
    std::fs::create_dir_all(&workspace).expect("workspace");
    env.set("XDG_CONFIG_HOME", config.display().to_string());
    (temp, workspace, config)
}

pub(super) async fn run_remote_check(workspace: &Path) -> Option<String> {
    let label = TaskLabel {
        package: "//".into(),
        name: "check".into(),
    };
    let spec = remote_workspace(workspace, std::slice::from_ref(&label));
    let summary = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect("remote run");

    summary
        .results
        .get(&label)
        .and_then(|result| result.remote_node_id.clone())
}
