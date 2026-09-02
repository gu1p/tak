use anyhow::{Result, bail};
use tak_core::v2::{
    Affinity, Execution, LocalExecution, RemoteExecution, RemoteSelection, Session, SessionReuse,
};

use super::super::v2_wire as wire;
use super::super::v2_wire_primitives::convert_output;
use super::context::convert_context;
use super::runtime::convert_container;

pub(super) fn validate_task_affinity(
    execution: Option<&Execution>,
    direct_session: Option<&Session>,
    task_affinity: Option<&Affinity>,
) -> Result<()> {
    if let Some(Execution::FirstAvailable { placements, .. }) = execution {
        for placement in placements {
            validate_task_affinity(Some(placement), direct_session, task_affinity)?;
        }
        return Ok(());
    }
    let attached = match execution {
        Some(Execution::LocalOnly { local }) => local.session.as_deref(),
        Some(Execution::RemoteOnly { remote }) => remote.session.as_deref(),
        Some(Execution::FirstAvailable { .. }) => unreachable!(),
        None => None,
    };
    if let Some(session) = direct_session.or(attached) {
        session.effective_affinity(task_affinity)?;
    }
    Ok(())
}

pub(super) fn validate_first_available(execution: Option<&Execution>) -> Result<()> {
    let Some(Execution::FirstAvailable { placements, .. }) = execution else {
        return Ok(());
    };
    let selections = placements
        .iter()
        .filter_map(Execution::remote)
        .map(|remote| remote.selection);
    if !all_equal(selections) {
        bail!("Execution.FirstAvailable requires every remote tier to use the same RemoteSelection")
    }
    let expected_runtime = placements.first().and_then(Execution::runtime);
    if placements
        .iter()
        .skip(1)
        .any(|placement| placement.runtime() != expected_runtime)
    {
        bail!(
            "Execution.FirstAvailable requires every tier to use the same container runtime; add the same container=... to every tier or remove it from all tiers"
        )
    }
    Ok(())
}

fn all_equal<T: PartialEq>(mut values: impl Iterator<Item = T>) -> bool {
    let Some(first) = values.next() else {
        return true;
    };
    values.all(|value| value == first)
}

pub(super) fn convert_execution(execution: wire::Execution) -> Result<Execution> {
    match execution {
        wire::Execution::LocalOnly { local } => Ok(Execution::LocalOnly {
            local: LocalExecution {
                reason: local.reason,
                session: local
                    .session
                    .map(|session| convert_session(*session))
                    .transpose()?
                    .map(Box::new),
                runtime: local.container.map(convert_container).transpose()?,
            },
        }),
        wire::Execution::RemoteOnly { remote } => Ok(Execution::RemoteOnly {
            remote: RemoteExecution {
                reason: remote.reason,
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
                runtime: remote.container.map(convert_container).transpose()?,
            },
        }),
        wire::Execution::FirstAvailable {
            policy_id,
            placements,
        } => {
            if policy_id.trim().is_empty() || placements.is_empty() {
                bail!("Execution.FirstAvailable requires a policy id and placements")
            }
            let placements = placements
                .into_iter()
                .map(convert_execution)
                .collect::<Result<Vec<_>>>()?;
            if placements
                .iter()
                .any(|placement| matches!(placement, Execution::FirstAvailable { .. }))
            {
                bail!("nested Execution.FirstAvailable is not supported")
            }
            Ok(Execution::FirstAvailable {
                policy_id,
                placements,
            })
        }
    }
}

pub(super) fn convert_session(session: wire::Session) -> Result<Session> {
    if session.kind != "session_v2" {
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
    validate_first_available(result.execution.as_deref())?;
    result.context = session.context.map(convert_context).transpose()?;
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
