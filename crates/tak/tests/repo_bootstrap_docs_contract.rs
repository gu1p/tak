use std::fs;
use std::path::{Path, PathBuf};

const FORBIDDEN_TOKENS: [&str; 5] = [
    "tak exec --",
    "cargo run --locked -p tak -- run",
    "make check",
    "GH_TOKEN",
    "GITHUB_TOKEN",
];

#[test]
fn repo_docs_use_system_tak_bootstrap_language() {
    let root = repo_root();
    let readme = load_text(&root.join("README.md"));
    let agents = load_text(&root.join("AGENTS.md"));

    assert!(
        readme.contains("system `tak` already on `PATH`"),
        "README.md should explain the repo uses the system tak command"
    );
    assert!(
        readme.contains("`./get-tak.sh`"),
        "README.md should explain the repo bootstrap command"
    );
    assert!(
        agents.contains("system `tak` already on `PATH`"),
        "AGENTS.md should explain the repo uses the system tak command"
    );

    for token in FORBIDDEN_TOKENS {
        assert!(
            !readme.contains(token),
            "README.md should not mention `{token}`"
        );
        assert!(
            !agents.contains(token),
            "AGENTS.md should not mention `{token}`"
        );
    }
}

#[test]
fn bootstrap_scripts_do_not_reference_github_tokens() {
    let root = repo_root();
    let get_tak = load_text(&root.join("get-tak.sh"));
    let get_takd = load_text(&root.join("get-takd.sh"));

    for token in ["GH_TOKEN", "GITHUB_TOKEN"] {
        assert!(
            !get_tak.contains(token),
            "get-tak.sh should not mention `{token}`"
        );
        assert!(
            !get_takd.contains(token),
            "get-takd.sh should not mention `{token}`"
        );
    }
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("repo root should be two levels above tak crate")
        .to_path_buf()
}

fn load_text(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}
