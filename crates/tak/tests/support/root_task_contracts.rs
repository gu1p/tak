#![allow(dead_code)]

use std::path::Path;

use anyhow::Result;
use tak_core::label::parse_label;
use tak_core::model::TaskLabel;
use tak_core::v2::{AuthoredModule, AuthoredTask, Step};
use tak_loader::{LoadOptions, inspect_authored_root_module};

const CARGO_SHARED_ENV_SCRIPT: &str = "TAK_TEST_TMPDIR=\"/tmp/tak-tests-$TAK_RUN_ID-$TAK_JOB_ID\" \
&& mkdir -p \"$TAK_TEST_TMPDIR\" .tmp/cargo-home .tmp/cargo-target-local \
&& TMPDIR=\"$TAK_TEST_TMPDIR\" CARGO_HOME=\"$PWD/.tmp/cargo-home\" \
CARGO_TARGET_DIR=\"$PWD/.tmp/cargo-target-local\" \
CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 \
CARGO_BUILD_JOBS=2 exec \"$@\"";

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
}

pub fn load_root_module() -> Result<AuthoredModule> {
    Ok(inspect_authored_root_module(repo_root(), &LoadOptions::default())?.module)
}

pub fn parse(label: &str) -> TaskLabel {
    parse_label(label, "//").expect("task label")
}

pub fn task<'a>(module: &'a AuthoredModule, label: &str) -> &'a AuthoredTask {
    module
        .tasks
        .iter()
        .find(|task| task.name == label)
        .unwrap_or_else(|| panic!("missing {label}"))
}

pub fn cmd_steps(task: &AuthoredTask, task_name: &str) -> Vec<Vec<String>> {
    task.steps
        .iter()
        .map(|step| match step {
            Step::Cmd { argv, cwd, env } => {
                assert_eq!(cwd.as_deref(), Some("."), "{task_name} workspace cwd");
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

pub fn expected_cargo_argv(rows: &[&[&str]]) -> Vec<Vec<String>> {
    rows.iter()
        .map(|row| {
            let mut argv = vec![
                "sh".to_string(),
                "-c".to_string(),
                CARGO_SHARED_ENV_SCRIPT.to_string(),
                "tak-cargo".to_string(),
                "cargo".to_string(),
            ];
            argv.extend(row.iter().map(|arg| (*arg).to_string()));
            argv
        })
        .collect()
}
