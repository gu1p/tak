use crate::support;

use anyhow::{Context, Result};

use support::example_workspace::stage_example_workspace;
use support::examples_catalog::load_catalog;
use support::examples_run::run_example;
use support::examples_run_env::setup_example_run;

fn repo_root() -> &'static std::path::Path {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
}

#[test]
fn tor_catalog_example_injects_live_tor_probe_env_into_example_runner() -> Result<()> {
    let catalog = load_catalog()?;
    let entry = catalog
        .example
        .iter()
        .find(|entry| entry.name == "large/26_remote_tor_artifact_roundtrip")
        .context("missing large/26_remote_tor_artifact_roundtrip catalog entry")?;
    let temp = tempfile::tempdir()?;
    let workspace_root = temp.path().join("workspace");
    stage_example_workspace(&entry.name, &workspace_root);

    let context = setup_example_run(entry, temp.path(), &workspace_root)?;

    assert_eq!(
        context
            .env
            .get("TAK_TOR_PROBE_TIMEOUT_MS")
            .map(String::as_str),
        Some("300000")
    );
    assert_eq!(
        context
            .env
            .get("TAK_TOR_PROBE_BACKOFF_MS")
            .map(String::as_str),
        Some("1000")
    );
    Ok(())
}

#[test]
fn tak_live_tor_harness_extends_startup_session_timeout() -> Result<()> {
    let source = std::fs::read_to_string(repo_root().join("crates/tak/tests/support/live_tor.rs"))?;

    assert!(
        source.contains("TAKD_TOR_STARTUP_SESSION_TIMEOUT_MS"),
        "live Tor smoke harness must extend the startup session timeout, not just the probe timeout:\n{source}"
    );
    Ok(())
}

#[test]
fn tak_live_tor_harness_extends_recovery_self_probe_timeout() -> Result<()> {
    let source = std::fs::read_to_string(repo_root().join("crates/tak/tests/support/live_tor.rs"))?;

    assert!(
        source.contains("TAKD_TOR_RECOVERY_PROBE_TIMEOUT_MS"),
        "live Tor smoke harness must extend the recovery self-probe timeout so slow onion self-checks do not mark ready nodes recovering:\n{source}"
    );
    assert!(
        source.contains("TAKD_TOR_RECOVERY_PROBE_BACKOFF_MS"),
        "live Tor smoke harness must keep recovery self-probe retries aligned with live Tor retry backoff:\n{source}"
    );
    Ok(())
}

#[test]
fn tor_catalog_example_uses_simulated_container_runtime_for_harness() -> Result<()> {
    let catalog = load_catalog()?;
    let entry = catalog
        .example
        .iter()
        .find(|entry| entry.name == "large/26_remote_tor_artifact_roundtrip")
        .context("missing large/26_remote_tor_artifact_roundtrip catalog entry")?;

    assert!(
        entry.simulate_container_runtime,
        "catalog harness should not require nested container engines for tor examples"
    );
    Ok(())
}

#[test]
fn tor_catalog_example_runs_through_example_harness() -> Result<()> {
    let catalog = load_catalog()?;
    let entry = catalog
        .example
        .iter()
        .find(|entry| entry.name == "large/26_remote_tor_artifact_roundtrip")
        .context("missing large/26_remote_tor_artifact_roundtrip catalog entry")?;
    let temp = tempfile::tempdir()?;
    run_example(entry, temp.path())
}
