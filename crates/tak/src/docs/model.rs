use std::collections::BTreeMap;

use anyhow::{Context, Result};
use serde::Deserialize;

const CATALOG_TOML: &str = include_str!("../../../../examples/catalog.toml");

#[derive(Debug, Deserialize)]
pub(super) struct Catalog {
    pub(super) example: Vec<ExampleEntry>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ExampleEntry {
    pub(super) name: String,
    pub(super) run_target: String,
    #[serde(default)]
    pub(super) capabilities: Vec<String>,
    #[serde(default)]
    pub(super) use_when: String,
    #[serde(default)]
    pub(super) project_shapes: Vec<String>,
    #[serde(default)]
    pub(super) avoid_when: Vec<String>,
}

pub(super) struct EmbeddedExampleSources {
    pub(super) name: &'static str,
    pub(super) source_files: &'static [EmbeddedSourceFile],
}

pub(super) struct EmbeddedSourceFile {
    pub(super) path: &'static str,
    pub(super) body: &'static str,
}

include!(concat!(env!("OUT_DIR"), "/docs_dump_examples.rs"));

pub(super) fn load_catalog() -> Result<Catalog> {
    toml::from_str(CATALOG_TOML).context("failed to parse embedded examples catalog")
}

pub(super) fn documented_examples(catalog: &Catalog) -> Vec<&ExampleEntry> {
    catalog
        .example
        .iter()
        .filter(|entry| {
            !entry.capabilities.is_empty()
                && !entry.use_when.trim().is_empty()
                && !entry.project_shapes.is_empty()
        })
        .collect()
}

pub(super) fn documented_example_sources() -> BTreeMap<&'static str, &'static EmbeddedExampleSources>
{
    DOCUMENTED_EXAMPLE_SOURCES
        .iter()
        .map(|entry| (entry.name, entry))
        .collect::<BTreeMap<_, _>>()
}
