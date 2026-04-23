use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn coverage_script_builds_checkout_binaries_under_llvm_cov_environment() {
    let script = fs::read_to_string(repo_root().join("scripts/run_coverage.sh"))
        .expect("read scripts/run_coverage.sh");

    for needle in [
        "cargo llvm-cov clean --workspace",
        "cargo llvm-cov show-env --sh",
        "coverage_target_dir=\"${CARGO_TARGET_DIR:-${CARGO_LLVM_COV_TARGET_DIR:-target}}\"",
        "cargo build --all-features -p tak --bin tak",
        "cargo build --all-features -p takd --bin takd",
        "export TAK_TEST_TAK_BIN=\"${coverage_target_dir}/debug/tak\"",
        "export TAK_TEST_TAKD_BIN=\"${coverage_target_dir}/debug/takd\"",
        "cargo llvm-cov report",
    ] {
        assert!(
            script.contains(needle),
            "run_coverage.sh must contain `{needle}`:\n{script}"
        );
    }

    assert!(
        !script.contains("cargo llvm-cov \\\n  --workspace"),
        "run_coverage.sh should not combine environment setup and report generation in one direct cargo llvm-cov invocation anymore:\n{script}"
    );
    assert!(
        !script.contains("cargo llvm-cov report \\\n  --workspace \\\n  --all-features"),
        "run_coverage.sh should not pass feature-selection flags to cargo llvm-cov report:\n{script}"
    );
    assert!(
        !script.contains("${CARGO_TARGET_DIR}/debug"),
        "run_coverage.sh should not require CARGO_TARGET_DIR to be exported directly:\n{script}"
    );
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .expect("repo root")
}
