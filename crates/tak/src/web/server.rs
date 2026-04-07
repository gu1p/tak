use std::io::Write;

use anyhow::{Context, Result};
use axum::routing::get;
use axum::{Router, serve};
use tak_core::model::{TaskLabel, WorkspaceSpec};

use super::handlers::{
    app_js_handler, graph_json_handler, index_html_handler, styles_css_handler,
    vis_network_css_handler, vis_network_js_handler,
};
use super::payload::build_graph_payload;
use super::types_and_assets::WebState;

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
    let app = graph_router(graph_state(spec, target)?);

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

pub(super) fn graph_state(spec: &WorkspaceSpec, target: Option<&TaskLabel>) -> Result<WebState> {
    let payload = build_graph_payload(spec, target)?;
    Ok(WebState {
        graph_json: serde_json::to_string(&payload).context("failed to encode graph payload")?,
    })
}

pub(super) fn graph_router(state: WebState) -> Router {
    Router::new()
        .route("/", get(index_html_handler))
        .route("/app.js", get(app_js_handler))
        .route("/styles.css", get(styles_css_handler))
        .route("/vendor/vis-network.min.js", get(vis_network_js_handler))
        .route("/vendor/vis-network.min.css", get(vis_network_css_handler))
        .route("/graph.json", get(graph_json_handler))
        .with_state(state)
}

fn should_auto_open_browser() -> bool {
    should_auto_open_browser_for(cfg!(debug_assertions), env_flag_set("TAK_NO_BROWSER_OPEN"))
}

pub(super) fn should_auto_open_browser_for(debug_assertions: bool, disable_open: bool) -> bool {
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
