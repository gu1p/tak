use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn make_and_status_docs_describe_daemon_owned_v2_behavior() {
    let root = repo_root();
    let readme = load(&root.join("README.md"));
    let architecture = load(&root.join("crates/tak-make/ARCHITECTURE.md"));
    let example = load(&root.join("examples/medium/11_machine_lock_shared_ui/README.md"));

    for stale in [
        "`tak make` has no declared output paths yet",
        "graph declares no Tak output",
        "`tak status` is currently unsupported in the client-only CLI build",
    ] {
        assert!(
            !format!("{readme}\n{architecture}\n{example}").contains(stale),
            "documentation still contains stale claim `{stale}`"
        );
    }

    for current in [
        "hard-affined `SharedWorkspace`",
        "dependent promoted goal observes files",
        "safely materializes",
        "daemon-owned local activity",
    ] {
        assert!(
            format!("{readme}\n{architecture}\n{example}").contains(current),
            "documentation is missing daemon-owned v2 claim `{current}`"
        );
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repository root")
        .to_path_buf()
}

fn load(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}
