use std::collections::BTreeMap;

use anyhow::Result;

use crate::support;
use support::{run_tak_expect_success, write_tasks};

fn strip_ansi(value: &str) -> String {
    let mut plain = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '\u{1b}' {
            plain.push(ch);
            continue;
        }
        if chars.next_if_eq(&'[').is_none() {
            continue;
        }
        for next in chars.by_ref() {
            if ('@'..='~').contains(&next) {
                break;
            }
        }
    }
    plain
}

#[test]
fn list_renders_task_docs_and_explain_reports_them() -> Result<()> {
    let temp = tempfile::tempdir()?;
    write_tasks(
        temp.path(),
        r#"
build = task("build", doc="Compile the local workspace artifacts.", steps=[cmd("sh", "-c", "mkdir -p out && echo build > out/build.txt")])
lint = task("lint", deps=[":build"], steps=[cmd("sh", "-c", "echo lint > out/lint.txt")])
SPEC = module_spec(tasks=[build, lint])
SPEC
"#,
    )?;

    let env = BTreeMap::new();
    let list = strip_ansi(&run_tak_expect_success(temp.path(), &["list"], &env)?);
    assert!(
        list.contains("//:build\n  Compile the local workspace artifacts."),
        "list:\n{list}"
    );
    assert!(list.contains("//:lint [//:build]"), "list:\n{list}");
    assert!(!list.contains("//:lint [//:build]\n  "), "list:\n{list}");

    let explain = run_tak_expect_success(temp.path(), &["explain", "//:build"], &env)?;
    assert!(explain.contains("label: //:build"), "explain:\n{explain}");
    assert!(
        explain.contains("doc:\n  Compile the local workspace artifacts."),
        "explain:\n{explain}"
    );

    let explain = run_tak_expect_success(temp.path(), &["explain", "//:lint"], &env)?;
    assert!(explain.contains("doc: (none)"), "explain:\n{explain}");
    Ok(())
}

#[test]
fn list_and_explain_preserve_multiline_task_docs() -> Result<()> {
    let temp = tempfile::tempdir()?;
    write_tasks(
        temp.path(),
        r#"
release = task("release", doc="""Prepare the release bundle.
Publish the signed archives.""", steps=[cmd("sh", "-c", "mkdir -p out && echo release > out/release.txt")])
SPEC = module_spec(tasks=[release])
SPEC
"#,
    )?;

    let env = BTreeMap::new();
    let list = strip_ansi(&run_tak_expect_success(temp.path(), &["list"], &env)?);
    assert!(
        list.contains("//:release\n  Prepare the release bundle.\n  Publish the signed archives."),
        "list:\n{list}"
    );

    let explain = run_tak_expect_success(temp.path(), &["explain", "//:release"], &env)?;
    assert!(
        explain.contains("doc:\n  Prepare the release bundle.\n  Publish the signed archives."),
        "explain:\n{explain}"
    );
    Ok(())
}
