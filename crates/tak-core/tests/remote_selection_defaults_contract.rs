use serde_json::json;
use tak_core::model::{RemoteDef, RemoteSelectionDef, RemoteSelectionSpec};

#[test]
fn remote_execution_defaults_to_balanced_selection() {
    let remote: RemoteDef = serde_json::from_value(json!({})).expect("remote spec");
    assert_eq!(remote.selection, RemoteSelectionDef::Balanced);
}

#[test]
fn resolved_remote_selection_defaults_to_balanced() {
    assert_eq!(
        RemoteSelectionSpec::default(),
        RemoteSelectionSpec::Balanced
    );
}

#[test]
fn remote_selection_keeps_explicit_ordered_strategies_and_rejects_shuffle() {
    let balanced: RemoteSelectionDef =
        serde_json::from_value(json!({"kind": "balanced"})).expect("balanced");
    let sequential: RemoteSelectionDef =
        serde_json::from_value(json!({"kind": "sequential"})).expect("sequential");
    let round_robin: RemoteSelectionDef =
        serde_json::from_value(json!({"kind": "round_robin"})).expect("round robin");
    let shuffle = serde_json::from_value::<RemoteSelectionDef>(json!({"kind": "shuffle"}))
        .expect_err("Shuffle must not remain in the public model");

    assert_eq!(balanced, RemoteSelectionDef::Balanced);
    assert_eq!(sequential, RemoteSelectionDef::Sequential);
    assert_eq!(round_robin, RemoteSelectionDef::RoundRobin);
    assert!(shuffle.to_string().contains("unknown variant `shuffle`"));
}
