use std::fs;
use std::path::PathBuf;

use tak_make::{FilesystemMakefileReader, MakefileReader};

#[test]
fn filesystem_reader_uses_makes_default_file_precedence() {
    let workspace = tempfile::tempdir().expect("temporary workspace");
    fs::write(workspace.path().join("Makefile"), "from-makefile:\n")
        .expect("write ordinary Makefile");
    fs::write(workspace.path().join("GNUmakefile"), "from-gnu:\n")
        .expect("write preferred GNUmakefile");

    let source = FilesystemMakefileReader
        .read(workspace.path())
        .expect("read default makefile");

    assert_eq!(source.makefile_path, PathBuf::from("GNUmakefile"));
    assert_eq!(source.contents, "from-gnu:\n");
}

#[test]
fn filesystem_reader_prefers_lowercase_makefile_over_makefile() {
    let workspace = tempfile::tempdir().expect("temporary workspace");
    fs::write(workspace.path().join("Makefile"), "from-makefile:\n")
        .expect("write ordinary Makefile");
    fs::write(workspace.path().join("makefile"), "from-lowercase:\n")
        .expect("write lowercase makefile");

    let source = FilesystemMakefileReader
        .read(workspace.path())
        .expect("read default makefile");

    assert_eq!(source.makefile_path, PathBuf::from("makefile"));
    assert_eq!(source.contents, "from-lowercase:\n");
}

#[test]
fn filesystem_reader_reports_when_no_default_makefile_exists() {
    let workspace = tempfile::tempdir().expect("temporary workspace");

    let error = FilesystemMakefileReader
        .read(workspace.path())
        .expect_err("missing Makefile should fail");

    let message = error.to_string();
    assert!(message.contains("Makefile"), "{message}");
    assert!(
        message.contains(&workspace.path().display().to_string()),
        "{message}"
    );
}

#[test]
fn filesystem_reader_reports_selected_file_read_failures() {
    let workspace = tempfile::tempdir().expect("temporary workspace");
    fs::create_dir(workspace.path().join("GNUmakefile")).expect("create unreadable Makefile path");

    let error = FilesystemMakefileReader
        .read(workspace.path())
        .expect_err("selected Makefile read should fail");

    let message = error.to_string();
    assert!(message.contains("failed to read Makefile"), "{message}");
    assert!(message.contains("GNUmakefile"), "{message}");
}
