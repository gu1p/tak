//! Contract for repo-root task context declarations.

use std::path::Path;

use anyhow::Result;
use tak_core::label::parse_label;
use tak_core::model::IgnoreSourceSpec;
use tak_loader::{LoadOptions, load_workspace};

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
}

#[test]
fn repo_root_check_task_opts_into_gitignore_context() -> Result<()> {
    let spec = load_workspace(repo_root(), &LoadOptions::default())?;
    let label = parse_label("//:check", "//").expect("check label");
    let task = spec.tasks.get(&label).expect("check task");

    assert!(
        task.context
            .ignored
            .iter()
            .any(|source| matches!(source, IgnoreSourceSpec::GitIgnore)),
        "expected //:check to declare gitignore() in its context"
    );
    Ok(())
}
