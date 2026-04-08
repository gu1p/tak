use std::fs;
use std::path::{Path, PathBuf};

const HERO_EXAMPLES: [&str; 8] = [
    "small/01_hello_single_task",
    "small/04_cmd_with_env_and_cwd",
    "small/08_retry_fixed_fail_once",
    "medium/11_machine_lock_shared_ui",
    "medium/18_multi_package_monorepo",
    "large/24_full_feature_matrix_end_to_end",
    "large/25_remote_direct_build_and_artifact_roundtrip",
    "large/28_hybrid_local_remote_test_suite_failure_with_logs",
];

const HERO_SECTIONS: [&str; 6] = [
    "## Why This Matters",
    "## Copy-Paste Starter",
    "## Parameter Alternatives",
    "## Runbook",
    "## Expected Signals",
    "## Artifacts",
];

const ROOT_TOKENS: [&str; 23] = [
    "`tak list`",
    "`tak tree`",
    "`tak explain <label>`",
    "`tak graph [label] --format dot`",
    "`tak web [label]`",
    "`tak run <label...>`",
    "`--keep-going`",
    "`tak status`",
    "`tak remote add <token>`",
    "`tak remote list`",
    "`takd init`",
    "`takd serve`",
    "`takd status`",
    "`takd logs`",
    "`takd token show`",
    "`get-takd.sh`",
    "placement=",
    "remote_node=",
    "transport=",
    "reason=",
    "context_hash=",
    "runtime=",
    "runtime_engine=",
];

#[test]
fn root_readme_surfaces_full_cli_and_run_metadata() {
    let readme = load_text(&repo_root().join("README.md"));
    for token in ROOT_TOKENS {
        assert!(readme.contains(token), "README.md missing `{token}`");
    }
}

#[test]
fn examples_index_has_hero_path_and_reference_matrix() {
    let index = load_text(&repo_root().join("examples/README.md"));
    assert!(index.contains("Hero"), "missing hero section");
    assert!(index.contains("catalog.toml"), "missing catalog reference");
    for hero in HERO_EXAMPLES {
        assert!(index.contains(hero), "examples/README.md missing `{hero}`");
    }
}

#[test]
fn hero_readmes_include_copy_paste_and_parameter_alternatives() {
    let root = repo_root();
    for hero in HERO_EXAMPLES {
        let path = root.join("examples").join(hero).join("README.md");
        let body = load_text(&path);
        for section in HERO_SECTIONS {
            assert!(
                body.contains(section),
                "{} missing `{section}`",
                path.display()
            );
        }
        assert!(
            body.contains("```python"),
            "{} missing python block",
            path.display()
        );
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
