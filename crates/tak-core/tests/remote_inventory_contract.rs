use tak_core::remote_inventory::{
    RemoteInventory, RemoteRecord, load_remote_inventory_at,
    remote_inventory_path_from_config_home, save_remote_inventory_at,
};

#[path = "remote_inventory_contract/legacy.rs"]
mod legacy;

#[test]
fn remote_inventory_path_uses_tak_config_directory() {
    let root = std::path::Path::new("/tmp/tak-config-home");

    assert_eq!(
        remote_inventory_path_from_config_home(root),
        root.join("tak/remotes.toml")
    );
}

#[test]
fn missing_remote_inventory_loads_empty_v1_inventory() {
    let temp = tempfile::tempdir().expect("tempdir");
    let inventory = load_remote_inventory_at(&temp.path().join("tak/remotes.toml"))
        .expect("load missing inventory");

    assert_eq!(inventory.version, 1);
    assert!(inventory.remotes.is_empty());
}

#[test]
fn remote_inventory_round_trips_records() {
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("tak/remotes.toml");
    let inventory = RemoteInventory {
        version: 1,
        remotes: vec![RemoteRecord {
            node_id: "builder-a".to_string(),
            display_name: "Builder A".to_string(),
            base_url: "http://builder-a.onion".to_string(),
            bearer_token: "secret".to_string(),
            pools: vec!["build".to_string()],
            tags: vec!["linux".to_string()],
            capabilities: vec!["docker".to_string()],
            transport: "tor".to_string(),
            enabled: true,
        }],
    };

    save_remote_inventory_at(&path, &inventory).expect("save inventory");
    let loaded = load_remote_inventory_at(&path).expect("load inventory");
    assert_eq!(loaded, inventory);
}

#[test]
fn enabled_tor_remotes_excludes_disabled_and_direct_records() {
    let inventory = RemoteInventory {
        version: 1,
        remotes: vec![
            record("tor-enabled", "tor", true),
            record("tor-disabled", "tor", false),
            record("direct-enabled", "direct", true),
        ],
    };

    let nodes = inventory
        .enabled_tor_remotes()
        .map(|record| record.node_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(nodes, vec!["tor-enabled"]);
}

fn record(node_id: &str, transport: &str, enabled: bool) -> RemoteRecord {
    RemoteRecord {
        node_id: node_id.to_string(),
        display_name: node_id.to_string(),
        base_url: format!("http://{node_id}.example"),
        bearer_token: "secret".to_string(),
        pools: Vec::new(),
        tags: Vec::new(),
        capabilities: Vec::new(),
        transport: transport.to_string(),
        enabled,
    }
}
