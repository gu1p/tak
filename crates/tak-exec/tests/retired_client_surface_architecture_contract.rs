use std::path::Path;

#[test]
fn tak_exec_has_no_retired_client_tor_or_observation_cache() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let library = read(root, "src/lib.rs");
    let manifest = read(root, "Cargo.toml");

    for retired in [
        "mod client_observations;",
        "mod client_tor;",
        "default_client_tor_config",
        "load_remote_observation",
        "record_remote_observation",
        "write_remote_observation",
    ] {
        assert!(
            !library.contains(retired),
            "tak-exec must not compile or export retired client surface `{retired}`"
        );
    }
    assert!(
        !manifest.contains("arti-client"),
        "tak-exec must not carry the daemon-owned Arti client dependency"
    );
    for retired in ["src/client_observations.rs", "src/client_tor.rs"] {
        assert!(
            !root.join(retired).exists(),
            "tak-exec must not retain retired client module `{retired}`"
        );
    }
}

fn read(root: &Path, relative: &str) -> String {
    std::fs::read_to_string(root.join(relative)).expect(relative)
}
