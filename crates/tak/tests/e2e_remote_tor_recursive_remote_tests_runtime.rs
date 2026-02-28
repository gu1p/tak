//! Optional black-box E2E for recursive remote smoke tests in nested container runtime.

use std::fs;

use anyhow::Result;

#[allow(dead_code)]
mod support;
use support::recursive_e2e::{run_recursive_remote_task, skip_recursive_e2e_reason};

#[test]
fn e2e_remote_tor_recursive_remote_smoke_tests_run_in_nested_container() -> Result<()> {
    if let Some(reason) = skip_recursive_e2e_reason() {
        eprintln!("skipping recursive E2E smoke contract: {reason}");
        return Ok(());
    }

    let run = run_recursive_remote_task(
        "recursive_remote_smoke",
        "set -eu; mkdir -p out; if [ -f /.dockerenv ] || [ -f /run/.containerenv ] || grep -Eq '(docker|containerd|podman|kubepods)' /proc/1/cgroup; then echo nested-container > out/runtime-kind.txt; else echo host > out/runtime-kind.txt; fi; uname -s > out/uname.txt; echo smoke-ok > out/remote-smoke-ok.txt",
    )?;

    assert!(run.stdout.contains("placement=remote"));
    assert!(run.stdout.contains("transport=tor"));
    assert!(run.stdout.contains("runtime=containerized"));
    assert_eq!(
        fs::read_to_string(run.workspace_root.join("out/remote-smoke-ok.txt"))?.trim(),
        "smoke-ok"
    );
    assert_eq!(
        fs::read_to_string(run.workspace_root.join("out/runtime-kind.txt"))?.trim(),
        "nested-container"
    );

    Ok(())
}
