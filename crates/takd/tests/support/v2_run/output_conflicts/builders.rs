use std::collections::BTreeMap;
use std::num::NonZeroU32;

use tak_core::v2::{JobEdge, OutputSelector, ResolvedJob, ResolvedTaskUnit, Step};

pub(super) fn producer(
    template: &ResolvedTaskUnit,
    name: &str,
    job_id: &str,
    contents: &str,
) -> ResolvedTaskUnit {
    ResolvedTaskUnit {
        task_id: format!("//:{name}"),
        job_id: job_id.into(),
        dependencies: vec![],
        steps: vec![shell(&format!(
            "mkdir -p shared; printf {contents} > shared/value.txt"
        ))],
        outputs: vec![OutputSelector::Path {
            value: "shared/value.txt".into(),
        }],
        ..template.clone()
    }
}

pub(super) fn consumer(template: &ResolvedTaskUnit) -> ResolvedTaskUnit {
    ResolvedTaskUnit {
        task_id: "//:consume".into(),
        job_id: "job-consume".into(),
        dependencies: vec!["//:left".into(), "//:right".into()],
        steps: vec![shell("exit 99")],
        outputs: vec![],
        ..template.clone()
    }
}

pub(super) fn scheduled(template: &ResolvedJob, id: &str, task: &str) -> ResolvedJob {
    let mut job = template.clone();
    job.job_id = id.into();
    job.task_ids = vec![task.into()];
    job.retry.max_attempts = NonZeroU32::new(2).unwrap();
    job
}

pub(super) fn edge(from: &str, to: &str) -> JobEdge {
    JobEdge {
        dependency_job_id: from.into(),
        dependent_job_id: to.into(),
    }
}

fn shell(script: &str) -> Step {
    Step::Cmd {
        argv: vec!["/bin/sh".into(), "-c".into(), script.into()],
        cwd: None,
        env: BTreeMap::new(),
    }
}
