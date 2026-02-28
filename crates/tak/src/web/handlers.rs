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
