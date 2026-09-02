use std::num::NonZeroU32;

use tak_core::v2::{
    ContainerSource, ResourceRequest, RuntimeResources, Session, SessionReuse, TaskRuntime,
};

use crate::v2_resolved_run_support::sample_run;

const CPU: u64 = 2_000;
const MEMORY: u64 = 2 * 1024 * 1024 * 1024;

#[test]
fn container_runtime_resources_must_exactly_match_the_job_reservation() {
    for reservation in [
        request(CPU - 1, MEMORY, 1),
        request(CPU, MEMORY - 1, 1),
        request(CPU, MEMORY, 2),
    ] {
        let mut run = container_run(CPU, MEMORY);
        run.jobs[0].resources = reservation;

        let error = run.validate().unwrap_err().to_string();
        assert!(error.contains("resources"), "{error}");
    }
}

#[test]
fn native_tasks_require_zero_cpu_and_memory_with_one_execution_slot() {
    for reservation in [request(1, 0, 1), request(0, 1, 1), request(0, 0, 2)] {
        let mut run = sample_run();
        run.jobs[0].resources = reservation;

        let error = run.validate().unwrap_err().to_string();
        assert!(error.contains("resources"), "{error}");
    }
}

#[test]
fn every_task_in_a_fused_container_job_projects_the_same_resources() {
    let mut run = container_run(CPU, MEMORY);
    let mut second = run.tasks[0].clone();
    second.task_id = "//:second".into();
    run.jobs[0].task_ids.push(second.task_id.clone());
    run.jobs[0].session = Some(Session::new("fused", SessionReuse::Container, None).unwrap());
    run.tasks.push(second);
    run.validate().unwrap();

    run.tasks[1].runtime = Some(runtime(CPU + 1, MEMORY));
    let error = run.validate().unwrap_err().to_string();
    assert!(error.contains("resources"), "{error}");
}

fn container_run(cpu_millis: u64, memory_bytes: u64) -> tak_core::v2::ResolvedRun {
    let mut run = sample_run();
    run.tasks[0].runtime = Some(runtime(cpu_millis, memory_bytes));
    run.jobs[0].resources = request(cpu_millis, memory_bytes, 1);
    run
}

fn runtime(cpu_millis: u64, memory_bytes: u64) -> TaskRuntime {
    TaskRuntime::configured_container(
        ContainerSource::Image {
            image: "alpine:3.20".into(),
        },
        vec![],
        Default::default(),
        Some(RuntimeResources {
            cpu_millis,
            memory_bytes,
        }),
    )
    .unwrap()
}

fn request(cpu_millis: u64, memory_bytes: u64, slots: u32) -> ResourceRequest {
    ResourceRequest {
        cpu_millis,
        memory_bytes,
        execution_slots: NonZeroU32::new(slots).unwrap(),
    }
}
