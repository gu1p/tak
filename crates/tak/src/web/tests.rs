use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use tak_core::model::{CurrentStateSpec, ResolvedTask, RetryDef, TaskExecutionSpec, TaskLabel};

use super::*;

fn label(package: &str, name: &str) -> TaskLabel {
    TaskLabel {
        package: package.to_string(),
        name: name.to_string(),
    }
}

fn task(label: TaskLabel, deps: Vec<TaskLabel>) -> ResolvedTask {
    ResolvedTask {
        label,
        doc: String::new(),
        deps,
        steps: Vec::new(),
        needs: Vec::new(),
        queue: None,
        retry: RetryDef::default(),
        timeout_s: None,
        context: CurrentStateSpec::default(),
        execution: TaskExecutionSpec::default(),
        tags: Vec::new(),
    }
}

fn workspace_fixture() -> WorkspaceSpec {
    let a = label("//pkg", "a");
    let b = label("//pkg", "b");
    let c = label("//pkg", "c");
    let d = label("//pkg", "d");

    let mut tasks = BTreeMap::new();
    tasks.insert(a.clone(), task(a, vec![b.clone()]));
    tasks.insert(b.clone(), task(b, vec![c.clone()]));
    tasks.insert(c.clone(), task(c, Vec::new()));
    tasks.insert(d.clone(), task(d, Vec::new()));

    WorkspaceSpec {
        project_id: "project-test".to_string(),
        root: PathBuf::from("/tmp"),
        tasks,
        limiters: HashMap::new(),
        queues: HashMap::new(),
    }
}

#[test]
fn payload_without_target_contains_all_tasks() {
    let workspace = workspace_fixture();
    let payload = build_graph_payload(&workspace, None).expect("payload should be built");

    assert_eq!(payload.nodes.len(), 4);
    assert_eq!(payload.edges.len(), 2);
}

#[test]
fn payload_with_target_contains_transitive_dependencies() {
    let workspace = workspace_fixture();
    let target = label("//pkg", "a");
    let payload =
        build_graph_payload(&workspace, Some(&target)).expect("closure payload should be built");

    let node_ids = payload
        .nodes
        .iter()
        .map(|node| node.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(node_ids, vec!["pkg:a", "pkg:b", "pkg:c"]);
    assert_eq!(payload.edges.len(), 2);
    assert!(
        payload
            .edges
            .iter()
            .any(|edge| edge.from == "pkg:b" && edge.to == "pkg:a")
    );
}

#[test]
fn production_guard_disables_browser_open_in_debug_or_when_overridden() {
    assert!(!should_auto_open_browser_for(true, false));
    assert!(!should_auto_open_browser_for(false, true));
    assert!(should_auto_open_browser_for(false, false));
}
