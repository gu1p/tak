use bollard::models::ContainerSummary;

use super::super::engine::managed_containers;
use super::super::policy::ManagedContainer;

#[test]
fn managed_containers_parses_timeout_running_and_paused_state() {
    let running_summary = ContainerSummary {
        id: Some("c1".to_string()),
        created: Some(42),
        state: Some("running".to_string()),
        labels: Some(std::collections::HashMap::from([
            ("tak.owner".to_string(), "takd".to_string()),
            ("tak.node_id".to_string(), "builder-a".to_string()),
            ("tak.timeout_s".to_string(), "30".to_string()),
        ])),
        ..Default::default()
    };
    let paused_summary = ContainerSummary {
        id: Some("c2".to_string()),
        created: Some(7),
        state: Some("paused".to_string()),
        labels: Some(std::collections::HashMap::from([
            ("tak.owner".to_string(), "takd".to_string()),
            ("tak.node_id".to_string(), "builder-a".to_string()),
        ])),
        ..Default::default()
    };
    let parsed = managed_containers(&[running_summary, paused_summary], "builder-a");
    assert_eq!(
        parsed,
        vec![
            ManagedContainer {
                id: "c1".to_string(),
                created: 42,
                has_timeout: true,
                paused: false,
            },
            ManagedContainer {
                id: "c2".to_string(),
                created: 7,
                has_timeout: false,
                paused: true,
            },
        ]
    );
}

#[test]
fn managed_containers_treats_zero_or_missing_timeout_as_pausable() {
    let zero = ContainerSummary {
        id: Some("z".to_string()),
        created: Some(1),
        state: Some("running".to_string()),
        labels: Some(std::collections::HashMap::from([
            ("tak.owner".to_string(), "takd".to_string()),
            ("tak.node_id".to_string(), "builder-a".to_string()),
            ("tak.timeout_s".to_string(), "0".to_string()),
        ])),
        ..Default::default()
    };
    let missing = ContainerSummary {
        id: Some("m".to_string()),
        created: Some(2),
        state: Some("running".to_string()),
        labels: Some(std::collections::HashMap::from([
            ("tak.owner".to_string(), "takd".to_string()),
            ("tak.node_id".to_string(), "builder-a".to_string()),
        ])),
        ..Default::default()
    };
    let parsed = managed_containers(&[zero, missing], "builder-a");
    assert!(parsed.iter().all(|container| !container.has_timeout));
}
