use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use tak_core::v2::AuthoredModule;

use super::{
    LoadOptions, TASKS_FILE,
    authored_source::{AuthoredSpecVersion, classify_source},
    v2_includes,
    workspace_discovery::detect_workspace_root,
};

#[derive(Debug)]
pub struct V2AuthoredRoot {
    pub workspace_root: PathBuf,
    pub tasks_file: PathBuf,
    pub module: AuthoredModule,
}

pub fn inspect_authored_root_module(root: &Path, options: &LoadOptions) -> Result<V2AuthoredRoot> {
    let workspace_root = detect_workspace_root(root)?;
    let tasks_file = workspace_root.join(TASKS_FILE).canonicalize()?;
    let source = fs::read_to_string(&tasks_file)?;
    let AuthoredSpecVersion::V2 = classify_source(&tasks_file, &source)?;
    Ok(V2AuthoredRoot {
        module: v2_includes::evaluate(&workspace_root, &tasks_file, options)?,
        workspace_root,
        tasks_file,
    })
}
