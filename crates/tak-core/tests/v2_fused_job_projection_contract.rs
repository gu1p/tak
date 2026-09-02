use tak_core::v2::{ContainerSource, Session, SessionReuse, TaskRuntime};

use crate::v2_resolved_run_support::sample_run;

#[test]
fn a_multi_task_job_requires_container_session_reuse() {
    let error = fused_run(None, false).validate().unwrap_err().to_string();

    assert!(error.contains("multiple tasks"), "{error}");
    assert!(error.contains("SessionReuse.Container"), "{error}");
}

#[test]
fn a_container_fused_job_requires_one_identical_task_runtime() {
    let mut run = fused_run(Some(runtime("alpine:3.20")), true);
    run.tasks[1].runtime = Some(runtime("debian:bookworm"));

    let error = run.validate().unwrap_err().to_string();

    assert!(error.contains("runtime"), "{error}");
    assert!(error.contains("identical"), "{error}");
}

#[test]
fn valid_native_and_container_fused_jobs_remain_accepted() {
    fused_run(None, true).validate().unwrap();
    fused_run(Some(runtime("alpine:3.20")), true)
        .validate()
        .unwrap();
}

fn fused_run(runtime: Option<TaskRuntime>, container_session: bool) -> tak_core::v2::ResolvedRun {
    let mut run = sample_run();
    run.tasks[0].runtime = runtime.clone();
    let mut second = run.tasks[0].clone();
    second.task_id = "//:second".into();
    run.jobs[0].task_ids.push(second.task_id.clone());
    if container_session {
        run.jobs[0].session = Some(Session::new("fused", SessionReuse::Container, None).unwrap());
    }
    run.tasks.push(second);
    run
}

fn runtime(image: &str) -> TaskRuntime {
    TaskRuntime::container(ContainerSource::Image {
        image: image.into(),
    })
}
