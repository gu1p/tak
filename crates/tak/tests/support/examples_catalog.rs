#![allow(dead_code)]

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Catalog {
    pub example: Vec<ExampleEntry>,
}

#[derive(Debug, Deserialize)]
pub struct ExampleEntry {
    pub name: String,
    pub run_target: String,
    pub explain_target: String,
    pub expect_success: bool,
    pub requires_daemon: bool,
    #[serde(default)]
    pub remote_fixture: Option<String>,
    #[serde(default)]
    pub expect_stdout_contains: Vec<String>,
    #[serde(default)]
    pub expect_stderr_contains: Vec<String>,
    #[serde(default)]
    pub check_files: Vec<String>,
    #[serde(default)]
    pub check_file_contains: Vec<CheckFileContains>,
}

#[derive(Debug, Deserialize)]
pub struct CheckFileContains {
    pub path: String,
    pub contains: String,
}

pub fn load_catalog() -> Result<Catalog> {
    let path = repo_root().join("examples/catalog.toml");
    let body =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    toml::from_str(&body).with_context(|| format!("failed to parse {}", path.display()))
}

pub fn assert_no_failures(label: &str, failures: Vec<String>) {
    if failures.is_empty() {
        return;
    }
    panic!("{label} failed:\n{}", failures.join("\n\n"));
}

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .to_path_buf()
}
