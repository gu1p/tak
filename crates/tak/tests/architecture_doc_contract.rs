use std::fs;
use std::path::{Path, PathBuf};

mod tokens;

use tokens::{
    REMOVED_TOKENS, REQUIRED_TOKENS, TOR_CAPABILITY_TOKENS, TOR_FIRST_REMOVED_TOKENS,
    TOR_FIRST_TOKENS,
};

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
    for token in REMOVED_TOKENS.into_iter().chain(TOR_FIRST_REMOVED_TOKENS) {
        assert!(
            !architecture.contains(token),
            "ARCHITECTURE.md still contains removed wording `{token}`"
        );
    }
}

#[test]
fn architecture_docs_cover_tor_first_peer_network_contract() {
    let root = repo_root();
    let combined = [
        "ARCHITECTURE.md",
        "crates/tak/ARCHITECTURE.md",
        "crates/tak-core/ARCHITECTURE.md",
        "crates/tak-exec/ARCHITECTURE.md",
        "crates/takd/ARCHITECTURE.md",
    ]
    .into_iter()
    .map(|relative_path| load_text(&root.join(relative_path)))
    .collect::<Vec<_>>()
    .join("\n");

    for token in TOR_FIRST_TOKENS {
        assert!(
            combined.contains(token),
            "architecture docs missing Tor-first token `{token}`"
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
