use std::fs;
use std::path::{Path, PathBuf};

/// Resolves repository root from the tak crate's manifest directory.
pub(crate) fn repo_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("repository root should be two levels above tak crate")
        .to_path_buf()
}

/// Recursively collects `src/*.rs` files under the provided directory.
pub(crate) fn collect_rust_source_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir)
        .unwrap_or_else(|err| panic!("failed to read directory {}: {err}", dir.display()));

    for entry in entries {
        let entry = entry.unwrap_or_else(|err| {
            panic!("failed to read directory entry in {}: {err}", dir.display())
        });
        let path = entry.path();

        if path.is_dir() {
            collect_rust_source_files(&path, files);
            continue;
        }

        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        if !path
            .components()
            .any(|component| component.as_os_str() == "src")
        {
            continue;
        }

        files.push(path);
    }
}
