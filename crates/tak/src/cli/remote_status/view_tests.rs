use super::view::RemoteStatusView;
use super::{RemoteRecord, RemoteStatusResult};

#[test]
fn checking_view_starts_every_node_in_sorted_order() {
    let view = RemoteStatusView::checking(&[remote("builder-z"), remote("builder-a")], 1, true);

    assert_eq!(view.node_ids(), vec!["builder-a", "builder-z"]);
    assert_eq!(view.checking_count(), 2);
    assert!(view.completed_results().is_empty());
}

#[test]
fn completed_results_remain_sorted_after_out_of_order_finishes() {
    let mut view =
        RemoteStatusView::checking(&[remote("builder-z"), remote("builder-a")], 1, false);

    view.mark_complete(error_result("builder-z"));
    view.mark_complete(error_result("builder-a"));

    let node_ids = view
        .completed_results()
        .into_iter()
        .map(|result| result.remote.node_id)
        .collect::<Vec<_>>();
    assert_eq!(node_ids, vec!["builder-a", "builder-z"]);
    assert_eq!(view.checking_count(), 0);
    assert!(view.has_errors());
}

fn remote(node_id: &str) -> RemoteRecord {
    RemoteRecord {
        node_id: node_id.to_string(),
        display_name: node_id.to_string(),
        base_url: format!("http://{node_id}.example"),
        bearer_token: "secret".to_string(),
        pools: vec!["default".to_string()],
        tags: vec!["builder".to_string()],
        capabilities: vec!["linux".to_string()],
        transport: "direct".to_string(),
        enabled: true,
    }
}

fn error_result(node_id: &str) -> RemoteStatusResult {
    RemoteStatusResult {
        remote: remote(node_id),
        status: None,
        error: Some("node status failed with HTTP 401".to_string()),
    }
}
