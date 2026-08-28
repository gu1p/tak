use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;

use anyhow::Result;
use tak_core::model::{
    CurrentStateSpec, ResolvedTask, RetryDef, StepDef, TaskExecutionSpec, TaskLabel, WorkspaceSpec,
};
use tak_exec::{TaskOutputChunk, TaskOutputObserver, TaskStructuredStatusEvent};

#[derive(Default)]
pub(super) struct Events(pub(super) Mutex<Vec<TaskStructuredStatusEvent>>);

impl TaskOutputObserver for Events {
    fn observe_output(&self, _chunk: TaskOutputChunk) -> Result<()> {
        Ok(())
    }

    fn observe_structured_status(&self, event: TaskStructuredStatusEvent) -> Result<()> {
        self.0.lock().expect("events").push(event);
        Ok(())
    }
}

pub(super) fn workspace(root: &std::path::Path) -> WorkspaceSpec {
    WorkspaceSpec {
        project_id: "scheduler-view".into(),
        root: root.into(),
        tasks: BTreeMap::from([
            task("a", vec![]),
            task("b", vec![]),
            task("all", vec![label("a"), label("b")]),
        ]),
        sessions: BTreeMap::new(),
        limiters: HashMap::new(),
        queues: HashMap::new(),
    }
}

fn task(name: &str, deps: Vec<TaskLabel>) -> (TaskLabel, ResolvedTask) {
    let label = label(name);
    let steps = (name != "all").then(|| StepDef::Cmd {
        argv: vec!["sh".into(), "-c".into(), ":".into()],
        cwd: None,
        env: BTreeMap::new(),
    });
    (
        label.clone(),
        ResolvedTask {
            label,
            doc: String::new(),
            deps,
            steps: steps.into_iter().collect(),
            needs: vec![],
            queue: None,
            retry: RetryDef::default(),
            timeout_s: None,
            context: CurrentStateSpec::default(),
            outputs: vec![],
            container_runtime: None,
            execution: TaskExecutionSpec::default(),
            session: None,
            cascade_execution: false,
            tags: vec![],
        },
    )
}

pub(super) fn label(name: &str) -> TaskLabel {
    TaskLabel {
        package: "//".into(),
        name: name.into(),
    }
}
