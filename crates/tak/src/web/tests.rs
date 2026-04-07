use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use axum::body::to_bytes;
use axum::extract::State;
use axum::http::header;
use axum::response::IntoResponse;
use tak_core::model::{
    CurrentStateSpec, ResolvedTask, RetryDef, TaskExecutionSpec, TaskLabel, WorkspaceSpec,
};

use super::handlers::{
    app_js_handler, graph_json_handler, index_html_handler, styles_css_handler,
    vis_network_css_handler, vis_network_js_handler,
};
use super::payload::build_graph_payload;
use super::server::{graph_router, graph_state, should_auto_open_browser_for};

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
fn payload_with_unknown_target_returns_error() {
    let workspace = workspace_fixture();
    let missing = label("//pkg", "missing");
    let error = build_graph_payload(&workspace, Some(&missing)).expect_err("target should fail");
    let rendered = format!("{error:#}");
    assert!(rendered.contains("task not found"));
    assert!(rendered.contains("pkg:missing"));
}

#[test]
fn production_guard_disables_browser_open_in_debug_or_when_overridden() {
    assert!(!should_auto_open_browser_for(true, false));
    assert!(!should_auto_open_browser_for(false, true));
    assert!(should_auto_open_browser_for(false, false));
}

#[tokio::test]
async fn ui_routes_and_handlers_remain_available() {
    let state = graph_state(&workspace_fixture(), Some(&label("//pkg", "a"))).expect("graph state");
    let graph_json = state.graph_json.clone();
    let _ = graph_router(state.clone());

    let index = index_html_handler().await.into_response();
    assert_eq!(
        index.headers()[header::CONTENT_TYPE],
        "text/html; charset=utf-8"
    );
    let app_js = app_js_handler().await.into_response();
    assert_eq!(
        app_js.headers()[header::CONTENT_TYPE],
        "application/javascript; charset=utf-8"
    );
    let styles = styles_css_handler().await.into_response();
    assert_eq!(
        styles.headers()[header::CONTENT_TYPE],
        "text/css; charset=utf-8"
    );
    let vendor_js = vis_network_js_handler().await.into_response();
    assert_eq!(
        vendor_js.headers()[header::CONTENT_TYPE],
        "application/javascript; charset=utf-8"
    );
    let vendor_css = vis_network_css_handler().await.into_response();
    assert_eq!(
        vendor_css.headers()[header::CONTENT_TYPE],
        "text/css; charset=utf-8"
    );

    let graph = graph_json_handler(State(state)).await.into_response();
    assert_eq!(
        graph.headers()[header::CONTENT_TYPE],
        "application/json; charset=utf-8"
    );
    let body = to_bytes(graph.into_body(), usize::MAX)
        .await
        .expect("graph body");
    let body = String::from_utf8(body.to_vec()).expect("utf8 graph body");
    assert_eq!(body, graph_json);
    assert!(body.contains("\"pkg:a\""));
    assert!(body.contains("\"pkg:b\""));
}
