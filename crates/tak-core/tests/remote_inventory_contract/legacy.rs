use tak_core::remote_inventory::load_remote_inventory_at;

#[test]
fn remote_inventory_accepts_legacy_records_without_display_name() {
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("tak/remotes.toml");
    std::fs::create_dir_all(path.parent().expect("inventory parent")).expect("inventory parent");
    std::fs::write(
        &path,
        r#"
[[remotes]]
node_id = "builder-legacy"
base_url = "http://builder-legacy.onion"
bearer_token = "secret"
transport = "tor"
"#,
    )
    .expect("write legacy inventory");

    let inventory = load_remote_inventory_at(&path).expect("load legacy inventory");
    assert_eq!(inventory.remotes[0].node_id, "builder-legacy");
    assert_eq!(inventory.remotes[0].display_name, "");
}
