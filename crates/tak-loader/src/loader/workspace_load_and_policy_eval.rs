use std::path::Path;

use anyhow::Result;
use tak_core::model::WorkspaceSpec;

use super::{
    LoadOptions, inspect_authored_root_module, workspace_discovery::detect_workspace_root,
};

pub fn load_workspace(root: &Path, options: &LoadOptions) -> Result<WorkspaceSpec> {
    let workspace_root = detect_workspace_root(root)?;
    let v2 = inspect_authored_root_module(&workspace_root, options)?;
    super::v2_workspace_view::read_only(v2)
}
