use tak_core::model::{
    ContainerResourceLimitsSpec, ContainerRuntimeSourceSpec, RemoteRuntimeSpec, StepDef,
};
use tak_proto::NodeInfo;

use super::super::*;
use crate::daemon::remote::{RemoteNodeContext, RemoteRuntimeConfig};

pub(crate) fn poison_status_state(context: &RemoteNodeContext) {
    let state = context.shared_status_state();
    let _ = std::thread::spawn(move || {
        let _guard = state.lock().expect("status state lock");
        panic!("poison status state");
    })
    .join();
}

pub(super) fn remote_context() -> RemoteNodeContext {
    RemoteNodeContext::new(
        NodeInfo {
            node_id: "builder-a".into(),
            display_name: "builder-a".into(),
            base_url: "http://127.0.0.1:1".into(),
            healthy: true,
            pools: vec!["default".into()],
            tags: vec!["builder".into()],
            capabilities: vec!["linux".into()],
            transport: "direct".into(),
            transport_state: "ready".into(),
            transport_detail: String::new(),
        },
        "secret".into(),
        RemoteRuntimeConfig::for_tests(),
    )
}

pub(super) fn payload(task_run_id: &str) -> RemoteWorkerSubmitPayload {
    RemoteWorkerSubmitPayload {
        workspace_zip: Vec::new(),
        task_run_id: task_run_id.into(),
        task_label: "//:check".into(),
        attempt: 1,
        steps: vec![StepDef::Cmd {
            argv: vec!["true".into()],
            cwd: None,
            env: Default::default(),
        }],
        timeout_s: None,
        runtime: Some(RemoteRuntimeSpec::Containerized {
            source: ContainerRuntimeSourceSpec::Image {
                image: "alpine:3.20".into(),
            },
            resource_limits: Some(ContainerResourceLimitsSpec {
                cpu_cores: Some(1.0),
                memory_mb: Some(512),
            }),
        }),
        needs: Vec::new(),
        outputs: Vec::new(),
        session: None,
        fused_members: Vec::new(),
        origin: Some("task".into()),
        runtime_source: Some("image:alpine:3.20".into()),
        command: Some("true".into()),
        execution_label: None,
    }
}
