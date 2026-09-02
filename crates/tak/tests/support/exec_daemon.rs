#![allow(dead_code)]

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use tak_core::model::WorkspaceSpec;

use super::local_daemon::LocalDaemonGuard;

pub struct ExecDaemon {
    _guard: LocalDaemonGuard,
    environment: BTreeMap<String, String>,
}

impl ExecDaemon {
    pub fn spawn(temp: &Path, workspace: &Path) -> Self {
        let socket = PathBuf::from(".tmp")
            .join(temp.file_name().expect("temporary directory name"))
            .join("d.sock");
        let guard = LocalDaemonGuard::spawn(&socket, &empty_spec(workspace));
        Self {
            _guard: guard,
            environment: BTreeMap::from([
                ("TAKD_SOCKET".into(), "../d.sock".into()),
                ("XDG_STATE_HOME".into(), "../state".into()),
            ]),
        }
    }

    pub fn environment(&self) -> &BTreeMap<String, String> {
        &self.environment
    }

    pub fn environment_mut(&mut self) -> &mut BTreeMap<String, String> {
        &mut self.environment
    }
}

fn empty_spec(root: &Path) -> WorkspaceSpec {
    WorkspaceSpec {
        project_id: "exec-v2".into(),
        root: root.to_path_buf(),
        tasks: BTreeMap::new(),
        sessions: BTreeMap::new(),
        limiters: HashMap::new(),
        queues: HashMap::new(),
    }
}
