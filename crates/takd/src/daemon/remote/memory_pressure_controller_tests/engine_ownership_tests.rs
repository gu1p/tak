use bollard::models::ContainerSummary;

use super::super::engine::managed_containers;

#[test]
fn managed_containers_excludes_foreign_and_legacy_takd_containers() {
    let summary = |id: &str, node_id: Option<&str>| ContainerSummary {
        id: Some(id.to_string()),
        state: Some("running".to_string()),
        labels: Some(
            [("tak.owner".to_string(), "takd".to_string())]
                .into_iter()
                .chain(node_id.map(|node_id| ("tak.node_id".to_string(), node_id.to_string())))
                .collect(),
        ),
        ..Default::default()
    };

    let parsed = managed_containers(
        &[
            summary("owned", Some("builder-a")),
            summary("foreign", Some("builder-b")),
            summary("legacy-unscoped", None),
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
