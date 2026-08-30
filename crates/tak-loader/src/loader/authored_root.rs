use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use tak_core::v2::AuthoredModule;

use super::{
    LoadOptions, TASKS_FILE,
    authored_source::{AuthoredSpecVersion, classify_source},
    v2_module_eval,
    workspace_discovery::detect_workspace_root,
};

#[derive(Debug)]
pub enum AuthoredRootModule {
    LegacyBootstrap,
    V2(Box<V2AuthoredRoot>),
}

#[derive(Debug)]
pub struct V2AuthoredRoot {
    pub workspace_root: PathBuf,
    pub tasks_file: PathBuf,
    pub module: AuthoredModule,
}

pub fn inspect_authored_root_module(
    root: &Path,
    options: &LoadOptions,
) -> Result<AuthoredRootModule> {
    let workspace_root = detect_workspace_root(root)?;
    let tasks_file = workspace_root.join(TASKS_FILE).canonicalize()?;
    let source = fs::read_to_string(&tasks_file)?;
    match classify_source(&tasks_file, &source)? {
        AuthoredSpecVersion::LegacyBootstrap => Ok(AuthoredRootModule::LegacyBootstrap),
        AuthoredSpecVersion::V2 => Ok(AuthoredRootModule::V2(Box::new(V2AuthoredRoot {
            module: v2_module_eval::evaluate(&tasks_file, options)?,
            workspace_root,
            tasks_file,
        }))),
    }
}
