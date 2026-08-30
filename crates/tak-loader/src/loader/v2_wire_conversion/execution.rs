use anyhow::{Result, bail};
use tak_core::v2::{
    Affinity, Execution, LocalExecution, RemoteExecution, RemoteSelection, Session, SessionReuse,
};

use super::super::v2_wire as wire;
use super::super::v2_wire_primitives::convert_output;

pub(super) fn validate_task_affinity(
    execution: Option<&Execution>,
    direct_session: Option<&Session>,
    task_affinity: Option<&Affinity>,
) -> Result<()> {
    let attached = match execution {
        Some(Execution::LocalOnly { local }) => local.session.as_deref(),
        Some(Execution::RemoteOnly { remote }) => remote.session.as_deref(),
        None => None,
    };
    if let Some(session) = direct_session.or(attached) {
        session.effective_affinity(task_affinity)?;
    }
    Ok(())
}

pub(super) fn convert_execution(execution: wire::Execution) -> Result<Execution> {
    match execution {
        wire::Execution::LocalOnly { local } => {
            if local.container.is_some() {
                bail!("v2 local containers are not active in this build");
            }
            Ok(Execution::LocalOnly {
                local: LocalExecution {
                    session: local
                        .session
                        .map(|session| convert_session(*session))
                        .transpose()?
                        .map(Box::new),
                },
            })
        }
        wire::Execution::RemoteOnly { remote } => {
            if remote.container.is_some() {
                bail!("v2 remote containers are not active in this build");
            }
            Ok(Execution::RemoteOnly {
                remote: RemoteExecution {
                    pool: remote.pool,
                    required_tags: remote.required_tags,
                    required_capabilities: remote.required_capabilities,
                    transport: remote.transport.map(convert_transport),
                    selection: convert_selection(remote.selection),
                    session: remote
                        .session
                        .map(|session| convert_session(*session))
                        .transpose()?
                        .map(Box::new),
                },
            })
        }
    }
}

pub(super) fn convert_session(session: wire::Session) -> Result<Session> {
    if session.kind != "session_v2" || session.context.is_some() {
        bail!("invalid v2 session payload");
    }
    let reuse = convert_reuse(session.reuse)?;
    let affinity = session.affinity.map(convert_affinity).transpose()?;
    let display_name = session.name.clone().unwrap_or_else(|| session.id.clone());
    let mut result = Session::new(display_name, reuse, affinity)?;
    result.id = session.id;
    result.name = session.name;
    result.execution = session
        .execution
        .map(|execution| convert_execution(*execution))
        .transpose()?
        .map(Box::new);
    result.validate()?;
    Ok(result)
}

fn convert_reuse(reuse: wire::Reuse) -> Result<SessionReuse> {
    match reuse {
        wire::Reuse::Workspace => Ok(SessionReuse::Workspace),
        wire::Reuse::Paths { paths } => Ok(SessionReuse::Paths {
            paths: paths.into_iter().map(convert_output).collect(),
        }),
        wire::Reuse::SharedWorkspace { max_parallel_tasks } => {
            Ok(SessionReuse::shared_workspace(max_parallel_tasks)?)
        }
        wire::Reuse::Container => Ok(SessionReuse::Container),
    }
}

pub(super) fn convert_affinity(affinity: wire::Affinity) -> Result<Affinity> {
    match affinity {
        wire::Affinity::PreferSameNode { group } => Ok(Affinity::prefer_same_node(group)?),
        wire::Affinity::RequireSameNode { group } => Ok(Affinity::require_same_node(group)?),
    }
}

fn convert_selection(selection: wire::Selection) -> RemoteSelection {
    match selection {
        wire::Selection::Balanced => RemoteSelection::Balanced,
        wire::Selection::Sequential => RemoteSelection::Sequential,
        wire::Selection::RoundRobin => RemoteSelection::RoundRobin,
    }
}

fn convert_transport(transport: wire::Transport) -> String {
    match transport {
        wire::Transport::Direct => "direct",
        wire::Transport::Any => "any",
        wire::Transport::Tor => "tor",
    }
    .to_owned()
}
