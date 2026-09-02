use anyhow::Result;
use tak_core::v2::{ContainerSource, ResolvedJob, SessionReuse, TaskRuntime};

use super::super::constraints::Context;
use crate::daemon::cache_locality::path_content_key;
use crate::daemon::scheduler::SchedulerNode;

pub(super) fn present(
    node: &SchedulerNode,
    job: &ResolvedJob,
    workspace_fingerprint: &str,
    soft_affinity: bool,
    context: &Context<'_>,
) -> Result<bool> {
    if soft_affinity || node.cached_content.contains(workspace_fingerprint) {
        return Ok(true);
    }
    if cached_image_present(node, job, context) {
        return Ok(true);
    }
    cached_paths_present(node, job, context)
}

fn cached_image_present(node: &SchedulerNode, job: &ResolvedJob, context: &Context<'_>) -> bool {
    context
        .run
        .tasks
        .iter()
        .filter(|task| job.task_ids.contains(&task.task_id))
        .filter_map(|task| task.runtime.as_ref())
        .any(|runtime| match runtime {
            TaskRuntime::Container {
                source: ContainerSource::Image { image },
                ..
            } => node.cached_content.contains(&format!("image:{image}")),
            TaskRuntime::Container {
                source: ContainerSource::Dockerfile { .. },
                ..
            } => false,
        })
}

fn cached_paths_present(
    node: &SchedulerNode,
    job: &ResolvedJob,
    context: &Context<'_>,
) -> Result<bool> {
    let Some(session) = job.session.as_ref() else {
        return Ok(false);
    };
    if !matches!(session.reuse, SessionReuse::Paths { .. }) {
        return Ok(false);
    }
    let key = path_content_key(context.run_id, &node.node_id, &session.id)?;
    Ok(node.cached_content.contains(&key))
}
