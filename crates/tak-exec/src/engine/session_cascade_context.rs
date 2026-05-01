use tak_core::model::{LocalSpec, RemoteSpec, ResolvedTask, SessionUseSpec, TaskExecutionSpec};

use super::remote_models::TaskPlacement;

pub(crate) fn execution_with_root_context(
    execution: TaskExecutionSpec,
    task: &ResolvedTask,
) -> TaskExecutionSpec {
    match execution {
        TaskExecutionSpec::LocalOnly(mut local) => {
            apply_root_context_to_local(task, &mut local);
            TaskExecutionSpec::LocalOnly(local)
        }
        TaskExecutionSpec::RemoteOnly(mut remote) => {
            apply_root_context_to_remote(task, &mut remote);
            TaskExecutionSpec::RemoteOnly(remote)
        }
        other => other,
    }
}

pub(crate) fn session_for_cascade_root(
    task: &ResolvedTask,
    session: &SessionUseSpec,
) -> SessionUseSpec {
    let mut session = session.clone();
    if session.context.is_none() {
        session.context = Some(task.context.clone());
    }
    session
}

pub(crate) fn apply_root_context_to_session(task: &ResolvedTask, placement: &mut TaskPlacement) {
    let Some(session) = placement.session.as_mut() else {
        return;
    };
    if session.context.is_none() {
        session.context = Some(task.context.clone());
    }
    if let Some(local) = placement.local.as_mut() {
        apply_session_to_local(local, session);
    }
    if let Some(remote) = placement.remote.as_mut() {
        apply_session_to_remote(remote, session);
    }
}

fn apply_root_context_to_local(task: &ResolvedTask, local: &mut LocalSpec) {
    if let Some(session) = local.session.as_mut()
        && session.context.is_none()
    {
        session.context = Some(task.context.clone());
    }
}

fn apply_root_context_to_remote(task: &ResolvedTask, remote: &mut RemoteSpec) {
    if let Some(session) = remote.session.as_mut()
        && session.context.is_none()
    {
        session.context = Some(task.context.clone());
    }
}

fn apply_session_to_local(local: &mut LocalSpec, session: &SessionUseSpec) {
    if let Some(local_session) = local.session.as_mut()
        && local_session.name == session.name
    {
        *local_session = session.clone();
    }
}

fn apply_session_to_remote(remote: &mut RemoteSpec, session: &SessionUseSpec) {
    if let Some(remote_session) = remote.session.as_mut()
        && remote_session.name == session.name
    {
        *remote_session = session.clone();
    }
}
