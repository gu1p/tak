#![allow(dead_code)]

use std::path::Path;

use anyhow::Result;

use super::example_workspace::stage_example_workspace;
use super::examples_catalog::ExampleEntry;
use super::examples_run_assert::{assert_example_outputs, assert_example_run};
use super::examples_run_env::setup_example_run;

pub fn run_example(entry: &ExampleEntry, temp_root: &Path) -> Result<()> {
    let workspace_root = temp_root.join("workspace");
    stage_example_workspace(&entry.name, &workspace_root);

    let context = setup_example_run(entry, temp_root, &workspace_root)?;
    assert_example_run(entry, &workspace_root, &context.env)?;
    assert_example_outputs(entry, &workspace_root)
}
