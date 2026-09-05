use std::fs;
use std::path::Path;
use std::process::{Command, Output};

#[test]
fn staged_source_check_detects_test_attributes_without_ripgrep() {
    let output = check("#[test]\nfn misplaced_test() {}\n", true);
    assert!(
        !output.status.success(),
        "missing ripgrep must not hide violations"
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("contains test attributes"));
}

#[test]
fn staged_source_check_accepts_clean_sources_without_ripgrep() {
    let output = check("pub fn example() {}\n", true);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn staged_source_check_fails_when_its_parser_cannot_run() {
    let output = check("#[test]\nfn misplaced_test() {}\n", false);
    assert!(
        !output.status.success(),
        "an unavailable parser must not produce a pass"
    );
}

fn check(source: &str, with_awk: bool) -> Output {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("scripts")).unwrap();
    fs::create_dir_all(root.join("crates/example/src")).unwrap();
    fs::create_dir(root.join("bin")).unwrap();
    fs::write(root.join("crates/example/src/lib.rs"), source).unwrap();
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    fs::copy(
        repo.join("scripts/check_no_tests_in_src.sh"),
        root.join("scripts/check.sh"),
    )
    .unwrap();
    for tool in ["dirname", "find", "sort", "awk"] {
        if tool != "awk" || with_awk {
            std::os::unix::fs::symlink(format!("/usr/bin/{tool}"), root.join("bin").join(tool))
                .unwrap();
        }
    }
    Command::new("/bin/bash")
        .arg("scripts/check.sh")
        .env("PATH", root.join("bin"))
        .env("TAK_LINE_MODE", "all")
        .current_dir(root)
        .output()
        .unwrap()
}
