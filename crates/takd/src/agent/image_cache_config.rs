use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use sysinfo::{DiskRefreshKind, Disks};

use super::InitAgentOptions;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentImageCacheConfig {
    pub budget_gb: f64,
    #[serde(default = "default_mutable_tag_ttl_secs")]
    pub mutable_tag_ttl_secs: u64,
    #[serde(default = "default_image_cache_sweep_interval_secs")]
    pub sweep_interval_secs: u64,
    #[serde(default = "default_low_disk_min_free_percent")]
    pub low_disk_min_free_percent: f64,
    #[serde(default = "default_low_disk_min_free_gb")]
    pub low_disk_min_free_gb: f64,
}

impl AgentImageCacheConfig {
    pub fn runtime_config(
        &self,
        state_root: &Path,
    ) -> Result<crate::daemon::remote::RemoteImageCacheRuntimeConfig> {
        Ok(crate::daemon::remote::RemoteImageCacheRuntimeConfig {
            db_path: state_root.join("agent.sqlite"),
            budget_bytes: decimal_gb_to_bytes(self.budget_gb)?,
            mutable_tag_ttl_secs: self.mutable_tag_ttl_secs.max(1),
            sweep_interval_secs: self.sweep_interval_secs.max(1),
            low_disk_min_free_percent: self.low_disk_min_free_percent,
            low_disk_min_free_bytes: decimal_gb_to_bytes(self.low_disk_min_free_gb)?,
        })
    }
}

pub(super) fn resolve_init_image_cache_config(
    state_root: &Path,
    options: &InitAgentOptions<'_>,
) -> Result<AgentImageCacheConfig> {
    if options.image_cache_budget_percent.is_some() && options.image_cache_budget_gb.is_some() {
        bail!("configure either --image-cache-budget-percent or --image-cache-budget-gb, not both");
    }

    let budget_gb = if let Some(value) = options.image_cache_budget_gb {
        positive_decimal(value, "--image-cache-budget-gb")?
    } else if let Some(value) = options.image_cache_budget_percent {
        budget_gb_from_percent(
            state_root,
            positive_decimal(value, "--image-cache-budget-percent")?,
        )?
    } else if io::stdin().is_terminal() {
        interactive_image_cache_budget_gb(state_root)?
    } else {
        budget_gb_from_percent(state_root, 20.0)?
    };

    Ok(AgentImageCacheConfig {
        budget_gb,
        mutable_tag_ttl_secs: default_mutable_tag_ttl_secs(),
        sweep_interval_secs: default_image_cache_sweep_interval_secs(),
        low_disk_min_free_percent: default_low_disk_min_free_percent(),
        low_disk_min_free_gb: default_low_disk_min_free_gb(),
    })
}

fn interactive_image_cache_budget_gb(state_root: &Path) -> Result<f64> {
    let available_gb = available_bytes_for_path(state_root)? as f64 / 1_000_000_000.0;
    println!("Choose Tak image cache budget for this node.");
    println!("Current available storage near state root: {available_gb:.1}GB");
    println!("Options: 10%, 20%, 40%, or enter a custom value like 15% or 50GB.");
    print!("Image cache budget [20%]: ");
    io::stdout().flush().ok();
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .context("read image cache budget")?;
    let value = input.trim();
    if value.is_empty() {
        return budget_gb_from_percent(state_root, 20.0);
    }
    if let Some(percent) = value.strip_suffix('%') {
        return budget_gb_from_percent(
            state_root,
            positive_decimal(percent.trim().parse()?, "image cache budget percent")?,
        );
    }
    if let Some(gb) = value
        .strip_suffix("GB")
        .or_else(|| value.strip_suffix("gb"))
    {
        return positive_decimal(gb.trim().parse()?, "image cache budget GB");
    }
    positive_decimal(value.parse()?, "image cache budget GB")
}

fn budget_gb_from_percent(path: &Path, percent: f64) -> Result<f64> {
    let available_bytes = available_bytes_for_path(path)?;
    Ok((available_bytes as f64 * (percent / 100.0)) / 1_000_000_000.0)
}

fn available_bytes_for_path(path: &Path) -> Result<u64> {
    let mut disks = Disks::new_with_refreshed_list();
    disks.refresh_specifics(false, DiskRefreshKind::everything());
    let disks = disks
        .list()
        .iter()
        .map(|disk| DiskCandidate {
            mount_point: disk.mount_point().to_path_buf(),
            available_bytes: disk.available_space(),
        })
        .collect::<Vec<_>>();
    available_bytes_for_path_with_disks(path, &disks)
}

#[derive(Debug, Clone)]
struct DiskCandidate {
    mount_point: PathBuf,
    available_bytes: u64,
}

fn available_bytes_for_path_with_disks(path: &Path, disks: &[DiskCandidate]) -> Result<u64> {
    std::fs::create_dir_all(path)
        .with_context(|| format!("create state root {}", path.display()))?;
    let path = path
        .canonicalize()
        .with_context(|| format!("canonicalize state root {}", path.display()))?;
    let selected = disks
        .iter()
        .filter(|disk| path.starts_with(&disk.mount_point))
        .max_by_key(|disk| disk.mount_point.display().to_string().len())
        .or_else(|| disks.first())
        .ok_or_else(|| anyhow!("failed to inspect available storage for image cache budget"))?;
    Ok(selected.available_bytes)
}

fn decimal_gb_to_bytes(value: f64) -> Result<u64> {
    let value = positive_decimal(value, "image cache budget")?;
    Ok((value * 1_000_000_000.0).round() as u64)
}

fn positive_decimal(value: f64, field: &str) -> Result<f64> {
    if value.is_finite() && value > 0.0 {
        return Ok(value);
    }
    bail!("{field} must be a positive number")
}

fn default_mutable_tag_ttl_secs() -> u64 {
    86_400
}

fn default_image_cache_sweep_interval_secs() -> u64 {
    60
}

fn default_low_disk_min_free_percent() -> f64 {
    10.0
}

fn default_low_disk_min_free_gb() -> f64 {
    10.0
}

#[path = "image_cache_config_tests.rs"]
mod image_cache_config_tests;
