//! Optional E2E contract for proving containerized remote runtime executes in a real container.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use tak_core::model::{
    CurrentStateSpec, LimiterKey, QueueDef, RemoteRuntimeSpec, RemoteSelectionSpec, RemoteSpec,
    RemoteTransportKind, ResolvedTask, RetryDef, StepDef, TaskExecutionSpec, TaskLabel,
    WorkspaceSpec,
};
use tak_exec::{RunOptions, run_tasks};

#[path = "support/static_remote_v1_server.rs"]
mod static_remote_v1_server;
use static_remote_v1_server::StaticRemoteServer;

#[tokio::test]
async fn remote_container_runtime_runs_in_real_container_context_when_enabled() {
    if std::env::var("TAK_E2E_REAL_CONTAINER").ok().as_deref() != Some("1") {
        eprintln!("skipping: set TAK_E2E_REAL_CONTAINER=1 to run real container E2E contract");
        return;
    }
    assert!(
        std::env::var("TAK_TEST_HOST_PLATFORM").is_err(),
        "real-container contract must not run with simulated runtime platform overrides"
    );
    let image = std::env::var("TAK_E2E_REAL_CONTAINER_IMAGE")
        .unwrap_or_else(|_| "busybox:1.36".to_string());

    let temp = tempfile::tempdir().expect("tempdir");
    let marker = temp.path().join("container-proof.txt");
    let remote = StaticRemoteServer::spawn();
    let label = TaskLabel {
        package: "//apps/web".to_string(),
        name: "real_container_context".to_string(),
    };
    let task = ResolvedTask {
        label: label.clone(),
        doc: String::new(),
        deps: Vec::new(),
        steps: vec![StepDef::Cmd {
            argv: vec![
                "sh".to_string(),
                "-c".to_string(),
                format!(
                    "if [ -f /.dockerenv ] || grep -Eq '(docker|containerd|podman|kubepods)' /proc/1/cgroup; then echo containerized > '{}'; else echo host > '{}'; exit 17; fi",
                    marker.display(),
                    marker.display()
                ),
            ],
            cwd: None,
            env: BTreeMap::new(),
        }],
        needs: Vec::new(),
        queue: None,
        retry: RetryDef::default(),
        timeout_s: None,
        execution: TaskExecutionSpec::RemoteOnly(RemoteSelectionSpec::Single(RemoteSpec {
            id: "remote-container-real".to_string(),
            endpoint: Some(remote.endpoint()),
            transport_kind: RemoteTransportKind::DirectHttps,
            service_auth_env: None,
            runtime: Some(RemoteRuntimeSpec::Containerized { image }),
        })),
        context: CurrentStateSpec::default(),
        tags: Vec::new(),
    };

    let mut tasks = std::collections::BTreeMap::new();
    tasks.insert(label.clone(), task);
    let spec = WorkspaceSpec {
        project_id: "project-test".to_string(),
        root: temp.path().to_path_buf(),
        tasks,
        limiters: HashMap::<LimiterKey, _>::new(),
        queues: HashMap::<LimiterKey, QueueDef>::new(),
    };

    let summary = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect("run should succeed");
    let result = summary.results.get(&label).expect("summary result");
    assert_eq!(result.remote_runtime_kind.as_deref(), Some("containerized"));
    assert_eq!(
        fs::read_to_string(&marker)
            .expect("real container marker should exist")
            .trim(),
        "containerized"
    );
}
