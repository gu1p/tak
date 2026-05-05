use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command as StdCommand, Output};

#[test]
fn generated_artifact_ignore_check_handles_staged_workspace_without_git_dir() {
    let temp = script_workspace("check_generated_artifacts_ignore.sh");
    fs::write(
        temp.path().join(".gitignore"),
        "/dist-manual/\n/.tmp/release-target/\n",
    )
    .expect("write gitignore");
    write_invalid_git_file(temp.path());

    assert_script_success(
        "generated artifact ignore check",
        run_script(temp.path(), "check_generated_artifacts_ignore.sh"),
    );
}

#[test]
fn line_limit_check_handles_staged_workspace_without_git_dir() {
    let temp = script_workspace("check_rust_file_limits.sh");
    write_small_crate(temp.path());
    write_invalid_git_file(temp.path());

    assert_script_success(
        "line limit check",
        run_script(temp.path(), "check_rust_file_limits.sh"),
    );
}

#[test]
fn src_test_separation_check_handles_staged_workspace_without_git_dir() {
    let temp = script_workspace("check_no_tests_in_src.sh");
    write_small_crate(temp.path());
    write_invalid_git_file(temp.path());

    assert_script_success(
        "src test separation check",
        run_script(temp.path(), "check_no_tests_in_src.sh"),
    );
}

fn script_workspace(script_name: &str) -> tempfile::TempDir {
    let temp = tempfile::tempdir().expect("tempdir");
    let scripts = temp.path().join("scripts");
    fs::create_dir_all(&scripts).expect("create scripts dir");
    fs::copy(
        repo_root().join("scripts").join(script_name),
        scripts.join(script_name),
    )
    .expect("copy script");
    temp
}

fn write_small_crate(root: &Path) {
    let src = root.join("crates/example/src");
    fs::create_dir_all(&src).expect("create src dir");
    fs::write(src.join("lib.rs"), "pub fn demo() {}\n").expect("write rust file");
}

fn write_invalid_git_file(root: &Path) {
    fs::write(root.join(".git"), "gitdir: /missing/worktree\n").expect("write git file");
}

fn run_script(root: &Path, script_name: &str) -> Output {
    StdCommand::new("bash")
        .arg(format!("scripts/{script_name}"))
        .current_dir(root)
        .output()
        .expect("run script")
}

fn assert_script_success(name: &str, output: Output) {
    assert!(
        output.status.success(),
        "{name} should pass from staged workspace\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .expect("repo root")
}
