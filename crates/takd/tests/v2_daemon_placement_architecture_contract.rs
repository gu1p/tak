use std::path::Path;

#[test]
fn daemon_has_no_in_memory_peer_placement_adapter() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let library = read(root, "src/lib.rs");
    let peer_manager = read(root, "src/daemon/peer_manager.rs");

    assert_absent(&library, &["PeerPlacementSelection"]);
    assert_absent(
        &peer_manager,
        &[
            "mod placement;",
            "PeerPlacementRequest",
            "PeerPlacementSelection",
            "placement_assignments",
            "round_robin_cursors",
            "select_placeable",
            "wait_for_placeable_peer",
        ],
    );
    assert!(
        !root.join("src/daemon/peer_manager/placement.rs").exists(),
        "daemon placement must remain in the durable v2 scheduler"
    );
}

fn assert_absent(source: &str, removed: &[&str]) {
    for symbol in removed {
        assert!(!source.contains(symbol), "legacy symbol remains: {symbol}");
    }
}

fn read(root: &Path, relative: &str) -> String {
    std::fs::read_to_string(root.join(relative)).expect(relative)
}
