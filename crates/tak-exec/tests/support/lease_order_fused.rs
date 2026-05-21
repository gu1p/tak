use tak_core::model::{
    ContainerResourceLimitsSpec, ContainerRuntimeSourceSpec, CurrentStateSpec, RemoteRuntimeSpec,
    ResolvedTask, RetryDef, SessionReuseSpec, SessionUseSpec, StepDef, TaskExecutionSpec,
    TaskLabel, WorkspaceSpec,
};

use super::{add_ui_lock_need, remote_builder_spec, shell_step};

pub fn fused_remote_cascade_spec(spec: &mut WorkspaceSpec) -> TaskLabel {
    let prepare = label("prepare");
    let check = label("check");
    let mut remote = remote_builder_spec(tak_core::model::RemoteTransportKind::Direct);
    remote.runtime = Some(container_runtime());
    remote.session = Some(container_session());
    spec.tasks.clear();
    spec.tasks.insert(
        prepare.clone(),
        task(
            prepare.clone(),
            Vec::new(),
            vec![shell_step("true")],
            TaskExecutionSpec::default(),
        ),
    );
    spec.tasks.insert(
        check.clone(),
        task(
            check.clone(),
            vec![prepare.clone()],
            vec![shell_step("true")],
            TaskExecutionSpec::RemoteOnly(remote),
        ),
    );
    spec.tasks.get_mut(&check).expect("check").cascade_execution = true;
    add_ui_lock_need(spec, &prepare);
    check
}

fn label(name: &str) -> TaskLabel {
    TaskLabel {
        package: "//".into(),
        name: name.into(),
    }
}

fn container_session() -> SessionUseSpec {
    SessionUseSpec {
        name: "container".into(),
        display_name: "container".into(),
        execution: None,
        reuse: SessionReuseSpec::Container,
        context: None,
    }
}

fn container_runtime() -> RemoteRuntimeSpec {
    RemoteRuntimeSpec::Containerized {
        source: ContainerRuntimeSourceSpec::Image {
            image: "alpine:3.20".into(),
        },
        resource_limits: Some(ContainerResourceLimitsSpec {
            cpu_cores: Some(1.0),
            memory_mb: Some(512),
        }),
    }
}

fn task(
    label: TaskLabel,
    deps: Vec<TaskLabel>,
    steps: Vec<StepDef>,
    execution: TaskExecutionSpec,
) -> ResolvedTask {
    ResolvedTask {
        label,
        doc: String::new(),
        deps,
        steps,
        needs: Vec::new(),
        queue: None,
        retry: RetryDef::default(),
        timeout_s: None,
        context: CurrentStateSpec::default(),
        outputs: Vec::new(),
        container_runtime: None,
        execution,
        session: None,
        cascade_execution: false,
        tags: Vec::new(),
    }
}
