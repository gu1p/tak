use anyhow::{Result, bail};
use tak_core::model::{SessionReuseSpec, SessionUseSpec, TaskExecutionSpec, TaskLabel};

use crate::engine::session_cascade::ExecutionCascadeOverride;

pub(super) fn uses_container_session(cascade: &ExecutionCascadeOverride) -> bool {
    execution_session(&cascade.execution)
        .or_else(|| {
            cascade
                .placement
                .as_ref()
                .and_then(|placement| placement.session.as_ref())
        })
        .is_some_and(|session| matches!(session.reuse, SessionReuseSpec::Container))
}

fn execution_session(execution: &TaskExecutionSpec) -> Option<&SessionUseSpec> {
    match execution {
        TaskExecutionSpec::LocalOnly(local) => local.session.as_ref(),
        TaskExecutionSpec::RemoteOnly(remote) => remote.session.as_ref(),
        _ => None,
    }
}

pub(super) fn validate_containerized_execution(
    root: &TaskLabel,
    execution: &TaskExecutionSpec,
) -> Result<()> {
    let has_container = match execution {
        TaskExecutionSpec::LocalOnly(local) => local.runtime.is_some(),
        TaskExecutionSpec::RemoteOnly(remote) => remote.runtime.is_some(),
        _ => false,
    };
    if has_container {
        return Ok(());
    }
    bail!(
        "task {} uses SessionReuse.Container but the selected execution is not containerized",
        super::builder::canonical_label(root)
    )
}
