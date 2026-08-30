use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::time::{Duration, Instant};

use tak_core::model::WorkspaceSpec;
use tak_proto::local_daemon::v2::RunLifecycleState;
use takd::RunStore;

pub(super) fn wait_for_running(store: &RunStore) -> String {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(run) = store.list_runs().unwrap().into_iter().next()
            && run.state == RunLifecycleState::Running
        {
            return run.run_id;
        }
        assert!(Instant::now() < deadline, "run never started");
        std::thread::sleep(Duration::from_millis(20));
    }
}

pub(super) fn wait_for_probe(path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !matches!(std::fs::read(path), Ok(bytes) if bytes == b"started") {
        assert!(Instant::now() < deadline, "attempt never started");
        std::thread::sleep(Duration::from_millis(20));
    }
}

pub(super) fn wait_for_terminal(store: &RunStore, run_id: &str) -> RunLifecycleState {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let state = store.summary(run_id).unwrap().unwrap().state;
        if state.is_terminal() || Instant::now() >= deadline {
            return state;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

pub(super) fn empty_spec(root: &Path) -> WorkspaceSpec {
    WorkspaceSpec {
        project_id: "cancel".into(),
        root: root.into(),
        tasks: BTreeMap::new(),
        sessions: BTreeMap::new(),
        limiters: HashMap::new(),
        queues: HashMap::new(),
    }
}

pub(super) const TASKS: &str = r#"SPEC = module_spec(spec_version=2, tasks=[task("slow", steps=[cmd("/bin/sh", "-c", "printf started > \"$PROBE\"; sleep 30; printf late >> \"$PROBE\"")])])
SPEC
"#;
