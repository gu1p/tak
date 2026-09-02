use serde_json::json;
use tak_core::v2::PlacementCandidate;

#[test]
fn placement_candidate_persists_resolved_remote_requirements() {
    let requirements = json!({
        "pool": "build",
        "required_tags": ["builder"],
        "required_capabilities": ["linux"],
        "transport": "direct",
    });
    let candidate: PlacementCandidate = serde_json::from_value(json!({
        "node_id": "worker-a",
        "kind": "remote",
        "transport": "direct",
        "reason": "matches authored requirements",
        "tier": 0,
        "requirements": requirements.clone(),
    }))
    .expect("protocol v2 candidate must accept resolved remote requirements");

    let encoded = serde_json::to_value(candidate).unwrap();
    assert_eq!(encoded["requirements"], requirements);
}
