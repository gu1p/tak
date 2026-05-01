use std::collections::BTreeMap;

use anyhow::Result;
use tak_core::model::{
    CurrentStateSpec, ExecutionPlacementSpec, PolicyDecisionSpec, SessionUseSpec,
    TaskExecutionSpec, TaskLabel,
};

use super::session_validation::validate_implicit_session_context;

pub(crate) fn validate_attached_execution_sessions(
    execution: &TaskExecutionSpec,
    contexts: &mut BTreeMap<String, CurrentStateSpec>,
    label: &TaskLabel,
    task_context: &CurrentStateSpec,
) -> Result<()> {
    match execution {
        TaskExecutionSpec::LocalOnly(local) => {
            validate_attached_session(local.session.as_ref(), contexts, label, task_context)
        }
        TaskExecutionSpec::RemoteOnly(remote) => {
            validate_attached_session(remote.session.as_ref(), contexts, label, task_context)
        }
        TaskExecutionSpec::ByExecutionPolicy { placements, .. } => {
            validate_placement_sessions(placements, contexts, label, task_context)
        }
        TaskExecutionSpec::ByCustomPolicy { decision, .. } => {
            validate_policy_decision_sessions(decision.as_ref(), contexts, label, task_context)
        }
        TaskExecutionSpec::UseSession { .. } => Ok(()),
    }
}

fn validate_placement_sessions(
    placements: &[ExecutionPlacementSpec],
    contexts: &mut BTreeMap<String, CurrentStateSpec>,
    label: &TaskLabel,
    task_context: &CurrentStateSpec,
) -> Result<()> {
    for placement in placements {
        let session = match placement {
            ExecutionPlacementSpec::Local(local) => local.session.as_ref(),
            ExecutionPlacementSpec::Remote(remote) => remote.session.as_ref(),
        };
        validate_attached_session(session, contexts, label, task_context)?;
    }
    Ok(())
}

fn validate_policy_decision_sessions(
    decision: Option<&PolicyDecisionSpec>,
    contexts: &mut BTreeMap<String, CurrentStateSpec>,
    label: &TaskLabel,
    task_context: &CurrentStateSpec,
) -> Result<()> {
    match decision {
        Some(PolicyDecisionSpec::Local { local, .. }) => validate_attached_session(
            local.as_ref().and_then(|local| local.session.as_ref()),
            contexts,
            label,
            task_context,
        ),
        Some(PolicyDecisionSpec::Remote { remote, .. }) => {
            validate_attached_session(remote.session.as_ref(), contexts, label, task_context)
        }
        None => Ok(()),
    }
}

fn validate_attached_session(
    session: Option<&SessionUseSpec>,
    contexts: &mut BTreeMap<String, CurrentStateSpec>,
    label: &TaskLabel,
    task_context: &CurrentStateSpec,
) -> Result<()> {
    let Some(session) = session else {
        return Ok(());
    };
    if session.context.is_some() {
        return Ok(());
    }
    validate_implicit_session_context(contexts, session.name.as_str(), label, task_context)
}
