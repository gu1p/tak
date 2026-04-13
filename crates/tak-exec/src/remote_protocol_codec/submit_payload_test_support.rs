use std::collections::BTreeMap;

use base64::Engine;
use tak_core::model::{
    CurrentStateSpec, Hold, LimiterRef, NeedDef, OutputSelectorSpec, PathAnchor, PathRef,
    RemoteRuntimeSpec, RemoteTransportKind, ResolvedTask, RetryDef, Scope, StepDef,
    TaskExecutionSpec, TaskLabel,
};

use super::*;

pub(super) fn direct_target(runtime: Option<RemoteRuntimeSpec>) -> StrictRemoteTarget {
    StrictRemoteTarget {
        node_id: "builder-a".into(),
        endpoint: "http://127.0.0.1:43123".into(),
        transport_kind: RemoteTransportKind::Direct,
        bearer_token: "secret".into(),
        runtime,
    }
}

pub(super) fn workspace(base64_zip: &str) -> RemoteWorkspaceStage {
    RemoteWorkspaceStage {
        temp_dir: tempfile::tempdir().expect("tempdir"),
        manifest_hash: "manifest".into(),
        archive_zip_base64: base64_zip.into(),
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
