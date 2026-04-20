use crate::support;

use anyhow::{Context, Result};

use support::example_workspace::stage_example_workspace;
use support::examples_catalog::load_catalog;
use support::examples_run::run_example;
use support::examples_run_env::setup_example_run;

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
