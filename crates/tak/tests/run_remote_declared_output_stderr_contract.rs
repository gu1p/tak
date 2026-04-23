//! Black-box contract for declared output failures after remote stderr activity.

use crate::support;

use std::fs;

use anyhow::Result;

use support::remote_declared_outputs::{attach_direct_remote, remote_env};
use support::{run_tak_expect_failure, write_tasks};

#[test]
fn run_remote_missing_declared_output_mentions_error_even_with_stderr_logs() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace_root = temp.path().join("workspace");
    let roots = support::live_direct::LiveDirectRoots::new(temp.path());
    fs::create_dir_all(&workspace_root)?;
    write_tasks(
        &workspace_root,
        r#"SPEC = module_spec(tasks=[task(
  "check",
  outputs=[path("out/missing.txt")],
  execution=RemoteOnly(Remote(pool="build", required_tags=["builder"], required_capabilities=["linux"], transport=DirectHttps(), runtime=ContainerRuntime(image="alpine:3.20"))),
  steps=[cmd("sh", "-c", "printf 'remote stderr\n' >&2")],
)])
SPEC
"#,
    )?;
    let _agent = attach_direct_remote(&workspace_root, &roots);

    let (_stdout, stderr) =
        run_tak_expect_failure(&workspace_root, &["run", "check"], &remote_env(&roots))?;

    assert!(
        stderr.contains("declared output path `out/missing.txt` was not created"),
        "stderr should keep the actual declared output failure:\n{stderr}"
    );
    Ok(())
}
