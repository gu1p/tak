//! Black-box E2E contract for retry and timeout behavior.

use std::fs;

use anyhow::Result;

#[allow(dead_code)]
use crate::support;
use support::exec_daemon::ExecDaemon;
use support::{run_tak_expect_failure, run_tak_expect_success, write_tasks};

#[test]
fn e2e_basic_retry_and_timeout_contract() -> Result<()> {
    fs::create_dir_all(".tmp")?;
    let temp = tempfile::tempdir_in(".tmp")?;
    let workspace = temp.path().join("workspace");
    let retry_out = workspace.join("out/retry.txt");

    write_tasks(
        &workspace,
        r#"
retry_task = task(
  "retry_task",
  retry=retry(attempts=2, on_exit=[42], backoff=fixed(0)),
  outputs=[path("out/retry.txt")],
  steps=[
    cmd("sh", "-c", "if [ \"$TAK_ATTEMPT\" = 1 ]; then exit 42; fi; mkdir -p out && echo recovered > out/retry.txt")
  ],
)
timeout_task = task(
  "timeout_task",
  timeout_s=1,
  steps=[cmd("sh", "-c", "sleep 2")],
)
SPEC = module_spec(spec_version=2, tasks=[retry_task, timeout_task])
SPEC
"#,
    )?;

    let daemon = ExecDaemon::spawn(temp.path(), &workspace);
    let env = daemon.environment();
    let run_retry = run_tak_expect_success(&workspace, &["run", "//:retry_task"], env)?;
    assert!(
        run_retry.contains("retrying tasks=//:retry_task"),
        "retry event missing:\n{run_retry}"
    );
    assert!(run_retry.contains("succeeded tasks=//:retry_task"));
    assert_eq!(fs::read_to_string(&retry_out)?.trim(), "recovered");

    let (stdout, stderr) = run_tak_expect_failure(&workspace, &["run", "//:timeout_task"], env)?;
    assert!(
        stdout.contains("timed out") || stdout.contains("timeout") || stderr.contains("timeout"),
        "timeout failure missing:\n{stdout}\n{stderr}"
    );

    Ok(())
}
