use anyhow::Result;
use tak_exec::{load_remote_observation_at, write_remote_observation_at};
use tak_proto::NodeInfo;

#[test]
fn observation_cache_round_trips_transport_state() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let state_root = temp.path().join("state");
    let node = NodeInfo {
        node_id: "builder-a".into(),
        display_name: "builder-a".into(),
        base_url: "http://builder-a.onion".into(),
        healthy: false,
        pools: vec!["build".into()],
        tags: vec!["builder".into()],
        capabilities: vec!["linux".into()],
        transport: "tor".into(),
        transport_state: "recovering".into(),
        transport_detail: "rendezvous accept failed".into(),
    };

    assert!(load_remote_observation_at(&state_root, "builder-a")?.is_none());
    write_remote_observation_at(&state_root, &node, 1_734_000_000_000)?;
    let observation = load_remote_observation_at(&state_root, "builder-a")?.expect("cached node");

    assert_eq!(observation.node_id, "builder-a");
    assert_eq!(observation.transport_state, "recovering");
    assert_eq!(observation.transport_detail, "rendezvous accept failed");
    Ok(())
}
