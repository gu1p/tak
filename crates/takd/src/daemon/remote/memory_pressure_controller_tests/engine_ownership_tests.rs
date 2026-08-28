use std::collections::HashMap;

use bollard::models::ContainerSummary;

use super::super::engine::managed_containers;
use crate::daemon::remote::container_ownership::labels_belong_to_node;

#[test]
fn managed_containers_excludes_foreign_and_legacy_takd_containers() {
    let summary = |id: &str, owner: &str, node_id: Option<&str>| ContainerSummary {
        id: Some(id.to_string()),
        state: Some("running".to_string()),
        labels: Some(
            [("tak.owner".to_string(), owner.to_string())]
                .into_iter()
                .chain(node_id.map(|node_id| ("tak.node_id".to_string(), node_id.to_string())))
                .collect(),
        ),
        ..Default::default()
    };

    let parsed = managed_containers(
        &[
            summary("owned", "takd-node-v1", Some("builder-a")),
            summary("foreign", "takd-node-v1", Some("builder-b")),
            summary("legacy-unscoped", "takd", None),
        ],
        "builder-a",
    );

    assert_eq!(
        parsed
            .into_iter()
            .map(|container| container.id)
            .collect::<Vec<_>>(),
        ["owned"]
    );
}

#[test]
fn scoped_ownership_rejects_the_legacy_global_owner_namespace() {
    let legacy = labels("takd", "builder-a");
    assert!(!labels_belong_to_node(Some(&legacy), "builder-a"));
}

#[test]
fn scoped_ownership_requires_the_new_namespace_and_matching_node() {
    let owned = labels("takd-node-v1", "builder-a");
    assert!(labels_belong_to_node(Some(&owned), "builder-a"));
    assert!(!labels_belong_to_node(Some(&owned), "builder-b"));
}

fn labels(owner: &str, node_id: &str) -> HashMap<String, String> {
    HashMap::from([
        ("tak.owner".to_string(), owner.to_string()),
        ("tak.node_id".to_string(), node_id.to_string()),
    ])
}
