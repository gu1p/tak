use std::collections::BTreeMap;

use base64::Engine;
use sha2::Digest;
use tak_core::model::{
    CurrentStateSpec, Hold, LimiterRef, NeedDef, OutputSelectorSpec, PathAnchor, PathRef,
    RemoteRuntimeSpec, ResolvedTask, RetryDef, Scope, StepDef, TaskExecutionSpec, TaskLabel,
};

use super::*;
use crate::engine::remote_models::StrictRemoteTransportKind;

pub(super) fn direct_target(runtime: Option<RemoteRuntimeSpec>) -> StrictRemoteTarget {
    StrictRemoteTarget {
        node_id: "builder-a".into(),
        endpoint: "http://127.0.0.1:43123".into(),
        transport_kind: StrictRemoteTransportKind::Direct,
        bearer_token: "secret".into(),
        runtime,
        remote_selection: tak_core::model::RemoteSelectionSpec::Sequential,
        required_pool: None,
        required_tags: Vec::new(),
        required_capabilities: Vec::new(),
        daemon_task_handle: None,
    }
}

pub(super) fn workspace(base64_zip: &str) -> RemoteWorkspaceStage {
    let archive = base64::engine::general_purpose::STANDARD
        .decode(base64_zip)
        .unwrap_or_default();
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let archive_path = temp_dir.path().join("workspace.zip");
    std::fs::write(&archive_path, &archive).expect("archive");
    RemoteWorkspaceStage {
        temp_dir,
        archive_path,
        archive_byte_len: archive.len() as u64,
        sha256: format!("{:x}", sha2::Sha256::digest(&archive)),
    }
}

pub(super) fn missing_archive_workspace() -> RemoteWorkspaceStage {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let archive_path = temp_dir.path().join("missing-workspace.zip");
    RemoteWorkspaceStage {
        temp_dir,
        archive_path,
        archive_byte_len: 0,
        sha256: format!("{:x}", sha2::Sha256::digest([])),
    }
}

pub(super) fn encoded_workspace() -> String {
    base64::engine::general_purpose::STANDARD.encode(b"zip-bytes")
}

pub(super) fn task_with_steps_and_needs() -> ResolvedTask {
    ResolvedTask {
        label: TaskLabel {
            package: "apps/web".into(),
            name: "build".into(),
        },
        doc: String::new(),
        deps: Vec::new(),
        steps: vec![
            StepDef::Cmd {
                argv: vec!["cargo".into(), "test".into()],
                cwd: Some("workspace".into()),
                env: BTreeMap::from([(String::from("RUST_LOG"), String::from("debug"))]),
            },
            StepDef::Script {
                path: "scripts/build.sh".into(),
                argv: vec!["--release".into()],
                interpreter: Some("bash".into()),
                cwd: Some("workspace".into()),
                env: BTreeMap::from([(String::from("CI"), String::from("1"))]),
            },
        ],
        needs: vec![
            need("cpu", Scope::Machine, None, 2.0),
            need("network", Scope::User, Some("builder"), 1.0),
            need("deploy", Scope::Project, Some("apps/web"), 3.0),
            need("disk", Scope::Worktree, None, 4.0),
        ],
        queue: None,
        retry: RetryDef::default(),
        timeout_s: Some(30),
        context: CurrentStateSpec::default(),
        outputs: vec![
            OutputSelectorSpec::Path(PathRef {
                anchor: PathAnchor::Workspace,
                path: "dist/out.txt".into(),
            }),
            OutputSelectorSpec::Glob {
                pattern: "reports/**/*.txt".into(),
            },
        ],
        container_runtime: None,
        execution: TaskExecutionSpec::default(),
        session: None,
        cascade_execution: false,
        tags: Vec::new(),
    }
}

fn need(name: &str, scope: Scope, scope_key: Option<&str>, slots: f64) -> NeedDef {
    NeedDef {
        limiter: LimiterRef {
            name: name.into(),
            scope,
            scope_key: scope_key.map(str::to_string),
        },
        slots,
        hold: Hold::During,
    }
}
