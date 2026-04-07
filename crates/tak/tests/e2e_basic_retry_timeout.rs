//! Black-box E2E contract for retry and timeout behavior.

use std::collections::BTreeMap;
use std::fs;

use anyhow::Result;

#[allow(dead_code)]
mod support;
use support::{run_tak_expect_failure, run_tak_expect_success, write_tasks};

#[test]
fn e2e_basic_retry_and_timeout_contract() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let retry_out = temp.path().join("out/retry.txt");

    write_tasks(
        temp.path(),
        &format!(
            r#"
retry_task = task(
  "retry_task",
  retry=retry(attempts=2, on_exit=[42], backoff=fixed(0)),
  steps=[
    cmd("sh", "-c", "mkdir -p out && if [ -f out/retry_seen ]; then echo recovered > {retry_out}; exit 0; else touch out/retry_seen; exit 42; fi")
  ],
)
timeout_task = task(
  "timeout_task",
  timeout_s=1,
  steps=[cmd("sh", "-c", "sleep 2")],
)
SPEC = module_spec(tasks=[retry_task, timeout_task])
SPEC
"#,
            retry_out = retry_out.display()
        ),
    )?;

    let env = BTreeMap::new();
    let run_retry = run_tak_expect_success(temp.path(), &["run", "apps/web:retry_task"], &env)?;
    assert!(run_retry.contains("apps/web:retry_task: ok (attempts=2"));
    assert_eq!(fs::read_to_string(&retry_out)?.trim(), "recovered");

    let (_stdout, stderr) =
        run_tak_expect_failure(temp.path(), &["run", "apps/web:timeout_task"], &env)?;
    assert!(
        stderr.contains("timed out") || stderr.contains("timeout"),
        "stderr should surface timeout failure, got: {stderr}"
    );

    Ok(())
}
