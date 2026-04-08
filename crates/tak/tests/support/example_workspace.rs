#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf()
}

pub fn stage_example_workspace(example_path: &str, destination: &Path) {
    let source = workspace_root().join("examples").join(example_path);
    copy_dir_all(&source, destination).unwrap_or_else(|err| {
        panic!(
            "failed to stage example {} into {}: {err}",
            source.display(),
            destination.display()
        )
    });
}

pub fn assert_file_contains(path: &Path, needle: &str, label: &str) {
    let body = fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {} {}: {err}", label, path.display()));
    assert!(
        body.contains(needle),
        "unexpected {label} contents:\n{body}"
    );
}

fn copy_dir_all(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}
