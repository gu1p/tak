#![allow(dead_code)]

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use tak_core::model::WorkspaceSpec;

use super::local_daemon::LocalDaemonGuard;

pub fn spawn(
    temp_root: &Path,
    workspace_root: &Path,
) -> (LocalDaemonGuard, BTreeMap<String, String>) {
    let socket = PathBuf::from(".tmp")
        .join(temp_root.file_name().expect("temporary directory name"))
        .join("d.sock");
    let guard = LocalDaemonGuard::spawn(&socket, &empty_spec(workspace_root));
    let environment = BTreeMap::from([
        ("TAKD_SOCKET".into(), "../d.sock".into()),
        ("XDG_STATE_HOME".into(), "../client-state".into()),
    ]);
    (guard, environment)
}

fn empty_spec(root: &Path) -> WorkspaceSpec {
    WorkspaceSpec {
        project_id: "v2-remote-test".into(),
        root: root.to_path_buf(),
        tasks: BTreeMap::new(),
        sessions: BTreeMap::new(),
        limiters: HashMap::new(),
        queues: HashMap::new(),
    }
}
