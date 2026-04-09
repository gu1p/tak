#![allow(dead_code)]

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

use super::examples_catalog::ExampleEntry;
use super::live_direct::{LiveDirectRoots, init_direct_agent, spawn_direct_agent, wait_for_token};
use super::live_direct_remote::add_remote as add_direct_remote;
use super::live_tor::{
    LiveTorRoots, init_tor_agent, spawn_tor_agent, wait_for_token as wait_for_tor_token,
};
use super::live_tor_remote::add_remote as add_tor_remote;
use super::tor_smoke::{ChildGuard, takd_bin};

pub fn remote_fixture(
    entry: &ExampleEntry,
    temp_root: &Path,
    workspace_root: &Path,
) -> Result<(Option<ChildGuard>, Option<ChildGuard>, Option<PathBuf>)> {
    let takd = takd_bin();
    match entry.remote_fixture.as_deref() {
        None => Ok((None, None, None)),
        Some("direct_http") => direct_fixture(entry, temp_root, workspace_root, &takd),
        Some("tor_onion_http") => tor_fixture(entry, temp_root, workspace_root, &takd),
        Some(other) => bail!("unsupported remote fixture {other} for {}", entry.name),
    }
}

fn expected_remote_node<'a>(entry: &'a ExampleEntry, fallback: &'a str) -> &'a str {
    entry
        .expect_stdout_contains
        .iter()
        .chain(&entry.expect_stderr_contains)
        .find_map(|line| line.strip_prefix("remote_node="))
        .unwrap_or(fallback)
}

fn direct_fixture(
    entry: &ExampleEntry,
    temp_root: &Path,
    workspace_root: &Path,
    takd: &Path,
) -> Result<(Option<ChildGuard>, Option<ChildGuard>, Option<PathBuf>)> {
    let roots = LiveDirectRoots::new(temp_root);
    init_direct_agent(
        takd,
        &roots,
        expected_remote_node(entry, "remote-direct-default"),
    );
    let agent = spawn_direct_agent(takd, &roots);
    let token = wait_for_token(takd, &roots);
    add_direct_remote(workspace_root, &roots, &token);
    Ok((Some(agent), None, Some(roots.client_config_root)))
}

fn tor_fixture(
    entry: &ExampleEntry,
    temp_root: &Path,
    workspace_root: &Path,
    takd: &Path,
) -> Result<(Option<ChildGuard>, Option<ChildGuard>, Option<PathBuf>)> {
    let roots = LiveTorRoots::new(temp_root);
    init_tor_agent(
        takd,
        &roots,
        expected_remote_node(entry, "remote-tor-default"),
    );
    let agent = spawn_tor_agent(takd, &roots);
    let token = wait_for_tor_token(takd, &roots);
    add_tor_remote(workspace_root, &roots, &token);
    Ok((None, Some(agent), Some(roots.client_config_root)))
}
