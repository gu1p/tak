use anyhow::{Result, bail};
use tak_core::model::{
    ExecutionPlacementSpec, RemoteSpec, ResolvedTask, SessionUseSpec, TaskExecutionSpec,
};

use super::placement::{PlacementCandidate, local_task_placement};
use super::placement_remote::remote_task_candidate;

pub(crate) fn resolve_session_candidates(
    task: &ResolvedTask,
    session: &SessionUseSpec,
    execution: &TaskExecutionSpec,
) -> Result<Vec<PlacementCandidate>> {
    match execution {
        TaskExecutionSpec::LocalOnly(local) => Ok(vec![PlacementCandidate::Ready(Box::new(
            local_task_placement(local_with_session(local.clone(), session), None),
        ))]),
        TaskExecutionSpec::RemoteOnly(remote) => Ok(vec![remote_task_candidate(
            task,
            &remote_with_session(remote.clone(), session),
            None,
        )?]),
        TaskExecutionSpec::ByExecutionPolicy { name, placements } => placements
            .iter()
            .map(|placement| {
                execution_policy_candidate_with_session(task, name, placement, session)
            })
            .collect(),
        TaskExecutionSpec::ByCustomPolicy { .. } => {
            bail!(
                "session `{}` uses unsupported ByCustomPolicy execution",
                session.name
            )
        }
        TaskExecutionSpec::UseSession { .. } => {
            bail!("session `{}` cannot use another session", session.name)
        }
    }
}

fn execution_policy_candidate_with_session(
    task: &ResolvedTask,
    policy_name: &str,
    placement: &ExecutionPlacementSpec,
    session: &SessionUseSpec,
) -> Result<PlacementCandidate> {
    match placement {
        ExecutionPlacementSpec::Local(local) => {
            Ok(PlacementCandidate::Ready(Box::new(local_task_placement(
                local_with_session(local.clone(), session),
                Some(policy_name.to_string()),
            ))))
        }
        ExecutionPlacementSpec::Remote(remote) => remote_task_candidate(
            task,
            &remote_with_session(remote.clone(), session),
            Some(policy_name.to_string()),
        ),
    }
}

fn local_with_session(
    mut local: tak_core::model::LocalSpec,
    session: &SessionUseSpec,
) -> tak_core::model::LocalSpec {
    if local.session.is_none() {
        local.session = Some(session.clone());
    }
    local
}

fn remote_with_session(mut remote: RemoteSpec, session: &SessionUseSpec) -> RemoteSpec {
    if remote.session.is_none() {
        remote.session = Some(session.clone());
    }
    remote
}
