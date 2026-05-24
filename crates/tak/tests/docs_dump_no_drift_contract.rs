use crate::support;
use anyhow::Result;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use support::run_tak_expect_success;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf()
}

fn run_docs_dump() -> Result<String> {
    let env = BTreeMap::new();
    run_tak_expect_success(&repo_root(), &["docs", "dump"], &env)
}

#[test]
fn docs_dump_includes_only_curated_verified_doctest_examples() -> Result<()> {
    let output = run_docs_dump()?;

    assert!(output.contains("## Verified Rust Examples"), "{output}");
    assert!(output.contains("#### Parse Tak task labels"), "{output}");
    assert!(output.contains("From `tak-core/src/label.rs`."), "{output}");
    assert!(
        output.contains("parse_label(\"apps/web:build\", \"//\")"),
        "{output}"
    );
    assert!(
        !output.contains("This behavior depends on internal state and is compile-checked only."),
        "{output}"
    );

    Ok(())
}

#[test]
fn docs_dump_keeps_authoring_workflow_owned_by_source_docs() -> Result<()> {
    let output = run_docs_dump()?;
    assert!(output.contains("## Authoring Workflow"), "{output}");
    assert!(
        output.contains("Start from the closest example and keep intent next to the source"),
        "{output}"
    );

    let renderer = fs::read_to_string(repo_root().join("crates/tak/src/docs.rs"))?;
    assert!(
        !renderer.contains("Start from the closest example and keep intent next to the source"),
        "authoring workflow prose must live in source docs, not docs.rs"
    );

    Ok(())
}
