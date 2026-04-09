#![allow(dead_code)]

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

pub fn write_tasks(root: &Path, body: &str) -> Result<()> {
    fs::create_dir_all(root)
        .with_context(|| format!("failed to create workspace root {}", root.display()))?;
    fs::write(root.join("TASKS.py"), body)
        .with_context(|| format!("failed to write TASKS.py under {}", root.display()))?;
    Ok(())
}
