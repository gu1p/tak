use super::support::{inventory, record};

#[test]
fn stale_reload_cannot_overwrite_a_daemon_inventory_mutation() {
    let manager =
        super::fixtures::peer_manager(inventory(vec![record("builder-a", "tor", true, "before")]));
    let stale_generation = manager.begin_inventory_reload();
    manager.apply_inventory(inventory(vec![record(
        "builder-a",
        "tor",
        true,
        "daemon-mutation",
    )]));

    manager.apply_inventory_reload(
        stale_generation,
        Ok(inventory(vec![record(
            "builder-a",
            "tor",
            true,
            "stale-read",
        )])),
    );

    let target = manager
        .connection_target("builder-a")
        .expect("configured peer");
    assert_eq!(target.bearer_token, "daemon-mutation");
}

#[test]
fn current_reload_still_applies_an_external_inventory_change() {
    let manager =
        super::fixtures::peer_manager(inventory(vec![record("builder-a", "tor", true, "before")]));
    let generation = manager.begin_inventory_reload();

    manager.apply_inventory_reload(
        generation,
        Ok(inventory(vec![record(
            "builder-a",
            "tor",
            true,
            "external-change",
        )])),
    );

    let target = manager
        .connection_target("builder-a")
        .expect("configured peer");
    assert_eq!(target.bearer_token, "external-change");
}
