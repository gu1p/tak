#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

use super::examples_catalog::ExampleEntry;
use super::examples_direct_fixture::direct_fixture;
use super::examples_tor_fixture::tor_fixture;
use super::tor_smoke::ChildGuard;

pub type RemoteFixtureSetup = (
    Option<ChildGuard>,
    Option<ChildGuard>,
    Option<PathBuf>,
    BTreeMap<String, String>,
);

pub fn remote_fixture(
    entry: &ExampleEntry,
    temp_root: &Path,
    workspace_root: &Path,
) -> Result<RemoteFixtureSetup> {
    match entry.remote_fixture.as_deref() {
        None => Ok((None, None, None, BTreeMap::new())),
        Some("direct_http") => direct_fixture(entry, temp_root, workspace_root),
        Some("tor_onion_http") => tor_fixture(entry, temp_root, workspace_root),
        Some(other) => bail!("unsupported remote fixture {other} for {}", entry.name),
    }
}

pub(super) fn expected_remote_node<'a>(entry: &'a ExampleEntry, fallback: &'a str) -> &'a str {
    entry
        .expect_stdout_contains
        .iter()
        .chain(&entry.expect_stderr_contains)
        .find_map(|line| line.strip_prefix("remote_node="))
        .unwrap_or(fallback)
}
