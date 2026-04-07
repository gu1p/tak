#![allow(dead_code)]

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

pub fn write_tasks(root: &Path, body: &str) -> Result<()> {
    fs::create_dir_all(root.join("apps/web"))
        .with_context(|| format!("failed to create apps/web under {}", root.display()))?;
    fs::write(root.join("apps/web/TASKS.py"), body)
        .with_context(|| format!("failed to write TASKS.py under {}", root.display()))?;
    Ok(())
}
