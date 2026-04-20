#![allow(dead_code)]

use std::path::Path;

use anyhow::Result;
use tak_core::label::parse_label;
use tak_core::model::{ResolvedTask, StepDef, TaskLabel, WorkspaceSpec};
use tak_loader::{LoadOptions, load_workspace};

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
}

pub fn load_root_spec() -> Result<WorkspaceSpec> {
    load_workspace(repo_root(), &LoadOptions::default())
}

pub fn parse(label: &str) -> TaskLabel {
    parse_label(label, "//").expect("task label")
}

pub fn cmd_steps(task: &ResolvedTask, task_name: &str) -> Vec<Vec<String>> {
    task.steps
        .iter()
        .map(|step| match step {
            StepDef::Cmd { argv, cwd, env } => {
                assert!(cwd.is_none(), "{task_name} should not override cwd");
                assert!(env.is_empty(), "{task_name} should not override env");
                argv.clone()
            }
            other => panic!("{task_name} should use cmd steps only: {other:?}"),
        })
        .collect()
}

pub fn expected_argv(rows: &[&[&str]]) -> Vec<Vec<String>> {
    rows.iter()
        .map(|row| row.iter().map(|arg| (*arg).to_string()).collect())
        .collect()
}
