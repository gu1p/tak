use serde::Serialize;

pub(super) const INDEX_HTML: &str = include_str!("../../assets/web/index.html");
pub(super) const APP_JS: &str = include_str!("../../assets/web/app.js");
pub(super) const STYLES_CSS: &str = include_str!("../../assets/web/styles.css");
pub(super) const VIS_NETWORK_JS: &str = include_str!("../../assets/vendor/vis-network.min.js");
pub(super) const VIS_NETWORK_CSS: &str = include_str!("../../assets/vendor/vis-network.min.css");

#[derive(Debug, Clone)]
pub(super) struct WebState {
    pub(super) graph_json: String,
}

#[derive(Debug, Serialize)]
pub(super) struct GraphPayload {
    pub(super) target: Option<String>,
    pub(super) nodes: Vec<GraphNode>,
    pub(super) edges: Vec<GraphEdge>,
}

#[derive(Debug, Serialize)]
pub(super) struct GraphNode {
    pub(super) id: String,
    pub(super) label: String,
    pub(super) package: String,
    pub(super) task: String,
    pub(super) deps: usize,
    pub(super) dependents: usize,
}

#[derive(Debug, Serialize)]
pub(super) struct GraphEdge {
    pub(super) from: String,
    pub(super) to: String,
}
