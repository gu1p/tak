use anyhow::Result;
use tak_core::v2::Execution;

use crate::support::root_task_contracts::{load_root_module, task};

#[test]
fn release_tasks_run_natively_inside_the_daemon_owned_local_worker() -> Result<()> {
    let module = load_root_module()?;
    for label in [
        "//:build-release-x86_64-unknown-linux-musl",
        "//:build-release-aarch64-unknown-linux-musl",
        "//:build-release-x86_64-apple-darwin",
        "//:build-release-aarch64-apple-darwin",
        "//:package-release-x86_64-unknown-linux-musl",
        "//:package-release-aarch64-unknown-linux-musl",
        "//:package-release-x86_64-apple-darwin",
        "//:package-release-aarch64-apple-darwin",
    ] {
        let execution = task(&module, label)
            .execution
            .as_ref()
            .unwrap_or_else(|| panic!("{label} should declare local execution"));
        let Execution::LocalOnly { local } = execution else {
            panic!("{label} should execute locally");
        };
        assert!(
            local.runtime.is_none(),
            "{label} must not require Docker or Podman on release runners"
        );
    }
    Ok(())
}
