use anyhow::{Result, bail};

use crate::support::examples_catalog::load_catalog;
use crate::support::examples_run::run_example;

#[test]
fn timeout_failure_example_remains_a_terminal_failure() -> Result<()> {
    let catalog = load_catalog()?;
    let Some(entry) = catalog
        .example
        .iter()
        .find(|entry| entry.name == "small/10_timeout_failure")
    else {
        bail!("small/10_timeout_failure is missing from the example catalog");
    };
    let temp = tempfile::tempdir()?;

    run_example(entry, temp.path())
}
