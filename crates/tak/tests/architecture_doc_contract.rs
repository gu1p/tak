use std::fs;
use std::path::{Path, PathBuf};

const REQUIRED_TOKENS: [&str; 9] = [
    "current directory",
    "`TASKS.py`",
    "`module_spec(includes=[...])`",
    "`takd init`",
    "`takd serve`",
    "`tak remote add <token>`",
    "`tak remote status`",
    "unix socket",
    "remote v1 HTTP",
];

const REMOVED_TOKENS: [&str; 3] = [
    "recursive module discovery",
    "discovers all `TASKS.py`",
    "`tak daemon start`",
];

const TOR_CAPABILITY_TOKENS: [&str; 5] = [
    "The Tor invite/address is a secret, not just a location.",
    "Anyone with it can submit jobs and read outputs/logs.",
    "Do not paste it into shared chats, issue trackers, screenshots, or logs.",
    "Rotate the onion address if exposed.",
    "Tak remote does not provide multi-user isolation.",
];

#[test]
fn root_architecture_matches_current_workspace_and_agent_model() {
    let architecture = load_text(&repo_root().join("ARCHITECTURE.md"));
    for token in REQUIRED_TOKENS {
        assert!(
            architecture.contains(token),
            "ARCHITECTURE.md missing `{token}`"
        );
    }
}

#[test]
fn root_architecture_drops_removed_workspace_and_daemon_wording() {
    let architecture = load_text(&repo_root().join("ARCHITECTURE.md"));
    for token in REMOVED_TOKENS {
        assert!(
            !architecture.contains(token),
            "ARCHITECTURE.md still contains removed wording `{token}`"
        );
    }
}

#[test]
fn tor_capability_model_is_documented_on_remote_surfaces() {
    let root = repo_root();
    for relative_path in [
        "README.md",
        "ARCHITECTURE.md",
        "crates/takd/ARCHITECTURE.md",
    ] {
        let body = load_text(&root.join(relative_path));
        for token in TOR_CAPABILITY_TOKENS {
            assert!(
                body.contains(token),
                "{relative_path} missing Tor capability wording `{token}`"
            );
        }
    }
}

fn repo_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.parent().and_then(Path::parent);
    root.expect("repo root should be two levels above tak crate")
        .to_path_buf()
}

fn load_text(path: &Path) -> String {
    let display = path.display();
    let message = |err| panic!("failed to read {display}: {err}");
    fs::read_to_string(path).unwrap_or_else(message)
}
