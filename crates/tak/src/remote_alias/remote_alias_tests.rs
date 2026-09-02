use super::remote_alias_for_node_id;

#[test]
fn alias_is_stable_and_human_readable() {
    assert_eq!(
        remote_alias_for_node_id("builder-node-123456"),
        remote_alias_for_node_id("builder-node-123456")
    );
    assert!(remote_alias_for_node_id("builder-node-123456").contains('-'));
}
