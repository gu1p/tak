use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;

use anyhow::{Context, Result, bail};
use axum::extract::State;
use axum::http::header;
use axum::routing::get;
use axum::{Router, serve};
use serde::Serialize;
use tak_core::model::{TaskLabel, WorkspaceSpec};

const INDEX_HTML: &str = include_str!("../../assets/web/index.html");
const APP_JS: &str = include_str!("../../assets/web/app.js");
const STYLES_CSS: &str = include_str!("../../assets/web/styles.css");
const VIS_NETWORK_JS: &str = include_str!("../../assets/vendor/vis-network.min.js");
const VIS_NETWORK_CSS: &str = include_str!("../../assets/vendor/vis-network.min.css");

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
