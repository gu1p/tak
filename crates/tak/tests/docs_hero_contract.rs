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

#[rustfmt::skip]
const ROOT_TOKENS: [&str; 32] = [
    "`tak list`", "`tak tree`", "`tak explain <label>`", "`tak graph [label] --format dot`",
    "`tak web [label]`", "`tak run <label...>`", "`tak run //:check`", "`tak run //:coverage`",
    "`--keep-going`", "`tak status`", "`tak remote add <token>`", "`tak remote list`",
    "`tak remote remove <node-id>`", "`tak remote logs --node <id>`", "`tak remote tasks --node <id>`",
    "`tak remote task logs --node <id> <task-run-id>`", "`tak task list`", "`tak task logs <task-run-id>`",
    "`takd init`", "`takd serve`", "`takd status`", "`takd logs`", "`takd token show`",
    "`./get-tak.sh`", "`get-takd.sh`", "placement=", "remote_node=", "transport=", "reason=",
    "context_hash=", "runtime=", "runtime_engine=",
];

#[test]
fn root_readme_surfaces_full_cli_and_run_metadata() {
    let readme = load_text(&repo_root().join("README.md"));
    for token in ROOT_TOKENS {
        assert!(readme.contains(token), "README.md missing `{token}`");
    }
}

#[test]
fn linked_phased_docs_page_exists() {
    let path = repo_root().join("docs/ergonomics-and-distribution-phases.md");

    assert!(path.is_file(), "{} should exist", path.display());
}

#[test]
fn example_docs_surface_hero_path_and_sections() {
    let index = load_text(&repo_root().join("examples/README.md"));
    assert!(index.contains("Hero"), "missing hero section");
    assert!(index.contains("catalog.toml"), "missing catalog reference");
    let root = repo_root();
    for hero in HERO_EXAMPLES {
        assert!(index.contains(hero), "examples/README.md missing `{hero}`");
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
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

fn load_text(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}
