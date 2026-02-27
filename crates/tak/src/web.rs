//! Embedded web graph visualization runtime for Tak.
//!
//! This module serves an interactive graph UI with fully embedded assets and opens a
//! browser tab in production builds.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;

use anyhow::{Context, Result, bail};
use axum::extract::State;
use axum::http::header;
use axum::routing::get;
use axum::{Router, serve};
use serde::Serialize;
use tak_core::model::{TaskLabel, WorkspaceSpec};

const INDEX_HTML: &str = include_str!("../assets/web/index.html");
const APP_JS: &str = include_str!("../assets/web/app.js");
const STYLES_CSS: &str = include_str!("../assets/web/styles.css");
const VIS_NETWORK_JS: &str = include_str!("../assets/vendor/vis-network.min.js");
const VIS_NETWORK_CSS: &str = include_str!("../assets/vendor/vis-network.min.css");

#[derive(Debug, Clone)]
struct WebState {
    graph_json: String,
}

#[derive(Debug, Serialize)]
struct GraphPayload {
    target: Option<String>,
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
}

#[derive(Debug, Serialize)]
struct GraphNode {
    id: String,
    label: String,
    package: String,
    task: String,
    deps: usize,
    dependents: usize,
}

#[derive(Debug, Serialize)]
struct GraphEdge {
    from: String,
    to: String,
}

/// Serves an interactive browser graph for the full workspace or one target closure.
///
/// ```no_run
/// # // Reason: This function starts a local HTTP server and waits for Ctrl+C.
/// # async fn demo(spec: &tak_core::model::WorkspaceSpec) -> anyhow::Result<()> {
/// use tak::web::serve_graph_ui;
///
/// serve_graph_ui(spec, None).await
/// # }
/// ```
pub async fn serve_graph_ui(spec: &WorkspaceSpec, target: Option<&TaskLabel>) -> Result<()> {
    let payload = build_graph_payload(spec, target)?;
    let state = WebState {
        graph_json: serde_json::to_string(&payload).context("failed to encode graph payload")?,
    };

    let app = Router::new()
        .route("/", get(index_html_handler))
        .route("/app.js", get(app_js_handler))
        .route("/styles.css", get(styles_css_handler))
        .route("/vendor/vis-network.min.js", get(vis_network_js_handler))
        .route("/vendor/vis-network.min.css", get(vis_network_css_handler))
        .route("/graph.json", get(graph_json_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .context("failed to bind web graph server")?;
    let address = listener
        .local_addr()
        .context("failed to resolve server local address")?;
    let url = format!("http://127.0.0.1:{}/", address.port());

    println!("web graph ui available at {url}");
    println!("press Ctrl+C to stop");
    let _ = std::io::stdout().flush();

    if should_auto_open_browser() {
        if let Err(err) = webbrowser::open(&url) {
            eprintln!("warning: failed to open browser automatically: {err}");
            eprintln!("open manually: {url}");
        }
    } else {
        eprintln!("info: browser auto-open disabled; open manually: {url}");
    }

    serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .context("web graph server exited with error")?;

    Ok(())
}

fn should_auto_open_browser() -> bool {
    should_auto_open_browser_for(cfg!(debug_assertions), env_flag_set("TAK_NO_BROWSER_OPEN"))
}

fn should_auto_open_browser_for(debug_assertions: bool, disable_open: bool) -> bool {
    !debug_assertions && !disable_open
}

fn env_flag_set(variable: &str) -> bool {
    std::env::var(variable)
        .ok()
        .map(|raw| {
            matches!(
                raw.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

fn build_graph_payload(spec: &WorkspaceSpec, target: Option<&TaskLabel>) -> Result<GraphPayload> {
    let selected = selected_labels(spec, target)?;
    let target_label = target.map(ToString::to_string);

    let mut edges = Vec::<GraphEdge>::new();
    let mut dependents = BTreeMap::<String, usize>::new();

    for label in &selected {
        let task = spec
            .tasks
            .get(label)
            .ok_or_else(|| anyhow::anyhow!("missing task for label {label}"))?;
        for dep in &task.deps {
            if selected.contains(dep) {
                edges.push(GraphEdge {
                    from: dep.to_string(),
                    to: label.to_string(),
                });
                *dependents.entry(dep.to_string()).or_insert(0) += 1;
            }
        }
    }

    edges.sort_by(|left, right| left.from.cmp(&right.from).then(left.to.cmp(&right.to)));

    let mut nodes = Vec::<GraphNode>::new();
    for label in &selected {
        let task = spec
            .tasks
            .get(label)
            .ok_or_else(|| anyhow::anyhow!("missing task for label {label}"))?;

        nodes.push(GraphNode {
            id: label.to_string(),
            label: label.to_string(),
            package: label.package.clone(),
            task: label.name.clone(),
            deps: task
                .deps
                .iter()
                .filter(|dep| selected.contains(*dep))
                .count(),
            dependents: dependents.get(&label.to_string()).copied().unwrap_or(0),
        });
    }

    nodes.sort_by(|left, right| left.id.cmp(&right.id));

    Ok(GraphPayload {
        target: target_label,
        nodes,
        edges,
    })
}

fn selected_labels(
    spec: &WorkspaceSpec,
    target: Option<&TaskLabel>,
) -> Result<BTreeSet<TaskLabel>> {
    let Some(target) = target else {
        return Ok(spec.tasks.keys().cloned().collect());
    };

    if !spec.tasks.contains_key(target) {
        bail!("task not found: {target}");
    }

    let mut selected = BTreeSet::<TaskLabel>::new();
    let mut stack = vec![target.clone()];

    while let Some(current) = stack.pop() {
        if !selected.insert(current.clone()) {
            continue;
        }

        let task = spec
            .tasks
            .get(&current)
            .ok_or_else(|| anyhow::anyhow!("task not found while walking closure: {current}"))?;
        for dep in &task.deps {
            stack.push(dep.clone());
        }
    }

    Ok(selected)
}

async fn index_html_handler() -> impl axum::response::IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        INDEX_HTML,
    )
}

async fn app_js_handler() -> impl axum::response::IntoResponse {
    (
        [(
            header::CONTENT_TYPE,
            "application/javascript; charset=utf-8",
        )],
        APP_JS,
    )
}

async fn styles_css_handler() -> impl axum::response::IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        STYLES_CSS,
    )
}

async fn vis_network_js_handler() -> impl axum::response::IntoResponse {
    (
        [(
            header::CONTENT_TYPE,
            "application/javascript; charset=utf-8",
        )],
        VIS_NETWORK_JS,
    )
}

async fn vis_network_css_handler() -> impl axum::response::IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        VIS_NETWORK_CSS,
    )
}

async fn graph_json_handler(State(state): State<WebState>) -> impl axum::response::IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
        state.graph_json,
    )
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashMap};
    use std::path::PathBuf;

    use tak_core::model::{ResolvedTask, RetryDef, TaskLabel};

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
        let payload = build_graph_payload(&workspace, Some(&target))
            .expect("closure payload should be built");

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
}
