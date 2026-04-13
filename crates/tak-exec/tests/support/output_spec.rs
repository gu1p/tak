#![allow(dead_code)]

use tak_core::model::{OutputSelectorSpec, PathAnchor, PathRef};

pub fn workspace_output_path(path: &str) -> OutputSelectorSpec {
    OutputSelectorSpec::Path(PathRef {
        anchor: PathAnchor::Workspace,
        path: path.into(),
    })
}

pub fn workspace_output_glob(pattern: &str) -> OutputSelectorSpec {
    OutputSelectorSpec::Glob {
        pattern: pattern.into(),
    }
}
