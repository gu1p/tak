//! Behavioral tests for loader discovery and module resolution.

use std::fs;

use tak_core::label::parse_label;
use tak_core::model::{
    PolicyDecisionSpec, RemoteRuntimeSpec, RemoteSelectionSpec, TaskExecutionSpec,
};
use tak_loader::{LoadOptions, detect_workspace_root, discover_tasks_files, load_workspace};

/// Ensures file discovery respects `.gitignore` filtering.
#[test]
fn discovers_tasks_files_respecting_gitignore() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(temp.path().join(".gitignore"), "ignored/\n").expect("write gitignore");

    fs::create_dir_all(temp.path().join("apps/web")).expect("mkdir");
    fs::create_dir_all(temp.path().join("ignored/hidden")).expect("mkdir");

    fs::write(
        temp.path().join("apps/web/TASKS.py"),
        "SPEC = {'spec_version': 1}\nSPEC\n",
    )
    .expect("write tasks");
    fs::write(
        temp.path().join("ignored/hidden/TASKS.py"),
        "SPEC = {'spec_version': 1}\nSPEC\n",
    )
    .expect("write ignored tasks");

    let files = discover_tasks_files(temp.path()).expect("discovery");
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("apps/web/TASKS.py"));
}

/// Ensures a loaded module yields fully-resolved workspace task labels.
#[test]
fn loads_module_and_resolves_labels() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("apps/web")).expect("mkdir");

    let code = r#"
SPEC = module_spec(
  tasks=[
    task("build", steps=[cmd("echo", "ok")]),
    task("test", deps=[":build"], steps=[cmd("echo", "test")])
  ]
)
SPEC
"#;
    fs::write(temp.path().join("apps/web/TASKS.py"), code).expect("write tasks");

    let spec = load_workspace(temp.path(), &LoadOptions::default()).expect("load");
    let build = parse_label("//apps/web:build", "//").expect("label");
    let test = parse_label("//apps/web:test", "//").expect("label");
    assert!(spec.tasks.contains_key(&build));
    assert!(spec.tasks.contains_key(&test));
}

/// Ensures dependency lists can reference another task object directly.
#[test]
fn loads_module_with_task_object_dependencies() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("apps/web")).expect("mkdir");

    let code = r#"
build = task("build", steps=[cmd("echo", "ok")])
test = task("test", deps=[build], steps=[cmd("echo", "test")])

SPEC = module_spec(tasks=[build, test])
SPEC
"#;
    fs::write(temp.path().join("apps/web/TASKS.py"), code).expect("write tasks");

    let spec = load_workspace(temp.path(), &LoadOptions::default()).expect("load");
    let build = parse_label("//apps/web:build", "//").expect("label");
    let test = parse_label("//apps/web:test", "//").expect("label");
    let test_task = spec.tasks.get(&test).expect("test task exists");

    assert!(spec.tasks.contains_key(&build));
    assert_eq!(test_task.deps, vec![build]);
}

/// Ensures a single dependency can be passed as a task object (without wrapping in a list).
#[test]
fn loads_module_with_single_task_object_dependency() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("apps/web")).expect("mkdir");

    let code = r#"
build = task("build", steps=[cmd("echo", "ok")])
test = task("test", deps=build, steps=[cmd("echo", "test")])

SPEC = module_spec(tasks=[build, test])
SPEC
"#;
    fs::write(temp.path().join("apps/web/TASKS.py"), code).expect("write tasks");

    let spec = load_workspace(temp.path(), &LoadOptions::default()).expect("load");
    let build = parse_label("//apps/web:build", "//").expect("label");
    let test = parse_label("//apps/web:test", "//").expect("label");
    let test_task = spec.tasks.get(&test).expect("test task exists");

    assert_eq!(test_task.deps, vec![build]);
}

/// Ensures workspace root detection ignores legacy `tak.toml` markers and prefers `.git`.
#[test]
fn detect_workspace_root_prefers_git_over_tak_toml() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join(".git")).expect("mkdir git");
    fs::create_dir_all(temp.path().join("workspace/apps/web")).expect("mkdir");
    fs::write(
        temp.path().join("workspace/tak.toml"),
        "project_id = \"legacy\"\n",
    )
    .expect("write tak.toml");

    let start = temp.path().join("workspace/apps/web");
    let root = detect_workspace_root(&start).expect("detect root");
    assert_eq!(
        root,
        temp.path()
            .canonicalize()
            .expect("canonicalize expected root")
    );
}

/// Ensures `module_spec(project_id=...)` in `TASKS.py` defines workspace identity.
#[test]
fn project_id_can_be_defined_in_tasks_module_spec() {
    let temp = tempfile::tempdir().expect("tempdir");

    let root_tasks = r#"
SPEC = module_spec(project_id="tasks-project", tasks=[])
SPEC
"#;
    fs::write(temp.path().join("TASKS.py"), root_tasks).expect("write root tasks");
    fs::create_dir_all(temp.path().join("apps/web")).expect("mkdir");
    fs::write(
        temp.path().join("apps/web/TASKS.py"),
        r#"
SPEC = module_spec(tasks=[task("build", steps=[cmd("echo", "ok")])])
SPEC
"#,
    )
    .expect("write package tasks");

    let spec = load_workspace(temp.path(), &LoadOptions::default()).expect("load");
    assert_eq!(spec.project_id, "tasks-project");
}

/// Ensures conflicting module-level project ids fail fast with a clear error.
#[test]
fn rejects_conflicting_module_project_ids() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("apps/a")).expect("mkdir");
    fs::create_dir_all(temp.path().join("apps/b")).expect("mkdir");
    fs::write(
        temp.path().join("apps/a/TASKS.py"),
        r#"
SPEC = module_spec(project_id="a-project", tasks=[task("build", steps=[cmd("echo", "a")])])
SPEC
"#,
    )
    .expect("write a tasks");
    fs::write(
        temp.path().join("apps/b/TASKS.py"),
        r#"
SPEC = module_spec(project_id="b-project", tasks=[task("test", steps=[cmd("echo", "b")])])
SPEC
"#,
    )
    .expect("write b tasks");

    let err = load_workspace(temp.path(), &LoadOptions::default()).expect_err("must fail");
    assert!(
        err.to_string().contains("conflicting project_id"),
        "unexpected error: {err}"
    );
}

/// Ensures legacy `tak.toml` no longer controls project id resolution.
#[test]
fn ignores_project_id_from_tak_toml() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(
        temp.path().join("tak.toml"),
        "project_id = \"legacy-config-id\"\n",
    )
    .expect("write tak.toml");
    fs::write(
        temp.path().join("TASKS.py"),
        r#"
SPEC = module_spec(tasks=[task("build", steps=[cmd("echo", "ok")])])
SPEC
"#,
    )
    .expect("write tasks");

    let spec = load_workspace(temp.path(), &LoadOptions::default()).expect("load");
    assert_ne!(spec.project_id, "legacy-config-id");
}

/// Ensures valid V1 execution constructors compile into expected runtime variants.
#[test]
fn maps_v1_execution_constructors_to_expected_ir_variants() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("apps/web")).expect("mkdir");
    fs::write(
        temp.path().join("apps/web/TASKS.py"),
        r#"
LOCAL = Local(id="dev-local", max_parallel_tasks=2)
REMOTE_A = Remote(id="remote-a", endpoint="http://127.0.0.1:8001")
REMOTE_B = Remote(id="remote-b", endpoint="http://127.0.0.1:8002")

SPEC = module_spec(tasks=[
  task("local_task", steps=[cmd("echo", "ok")], execution=LocalOnly(LOCAL)),
  task("remote_single", steps=[cmd("echo", "ok")], execution=RemoteOnly(REMOTE_A)),
  task("remote_list", steps=[cmd("echo", "ok")], execution=RemoteOnly([REMOTE_A, REMOTE_B])),
  task("policy_task", steps=[cmd("echo", "ok")], execution=ByCustomPolicy("policy_v1")),
])
SPEC
"#,
    )
    .expect("write tasks");

    let spec = load_workspace(temp.path(), &LoadOptions::default()).expect("load");

    let local_label = parse_label("//apps/web:local_task", "//").expect("local label");
    let local_task = spec.tasks.get(&local_label).expect("local task");
    match &local_task.execution {
        TaskExecutionSpec::LocalOnly(local) => {
            assert_eq!(local.id, "dev-local");
            assert_eq!(local.max_parallel_tasks, 2);
        }
        other => panic!("expected local execution, got: {other:?}"),
    }

    let remote_single_label = parse_label("//apps/web:remote_single", "//").expect("single label");
    let remote_single = spec.tasks.get(&remote_single_label).expect("single task");
    match &remote_single.execution {
        TaskExecutionSpec::RemoteOnly(RemoteSelectionSpec::Single(remote)) => {
            assert_eq!(remote.id, "remote-a");
            assert_eq!(remote.endpoint.as_deref(), Some("http://127.0.0.1:8001"));
        }
        other => panic!("expected strict single remote execution, got: {other:?}"),
    }

    let remote_list_label = parse_label("//apps/web:remote_list", "//").expect("list label");
    let remote_list = spec.tasks.get(&remote_list_label).expect("list task");
    match &remote_list.execution {
        TaskExecutionSpec::RemoteOnly(RemoteSelectionSpec::List(nodes)) => {
            let ids = nodes
                .iter()
                .map(|node| node.id.as_str())
                .collect::<Vec<_>>();
            assert_eq!(ids, vec!["remote-a", "remote-b"]);
        }
        other => panic!("expected fallback remote list execution, got: {other:?}"),
    }

    let policy_label = parse_label("//apps/web:policy_task", "//").expect("policy label");
    let policy_task = spec.tasks.get(&policy_label).expect("policy task");
    match &policy_task.execution {
        TaskExecutionSpec::ByCustomPolicy {
            policy_name,
            decision,
        } => {
            assert_eq!(policy_name, "policy_v1");
            assert!(
                decision.is_none(),
                "string policy should not compile decision"
            );
        }
        other => panic!("expected policy execution, got: {other:?}"),
    }
}

/// Ensures the canonical V1 import shape (`from tak ...`, `from tak.remote ...`) loads.
#[test]
fn loads_canonical_v1_import_surface() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("apps/web")).expect("mkdir");
    fs::write(
        temp.path().join("apps/web/TASKS.py"),
        r#"
from tak import cmd, module_spec, task, path, gitignore
from tak.remote import (
    Local, Remote,
    LocalOnly, RemoteOnly, ByCustomPolicy,
    PolicyContext, Decision, Reason,
    CurrentState,
    WorkspaceTransferMode, ResultSyncMode, results,
    RemoteTransportMode, ServiceAuth,
)

LOCAL = Local(id="dev-local", max_parallel_tasks=1)
REMOTE = Remote(
    id="remote-a",
    transport=RemoteTransportMode.DirectHttps(
        endpoint="https://127.0.0.1:8443",
        auth=ServiceAuth.from_env("TAK_NODE_REMOTE_A_TOKEN"),
    ),
    workspace=WorkspaceTransferMode.REPO_ZIP_SNAPSHOT,
    result=results(sync=ResultSyncMode.OUTPUTS_AND_LOGS),
)

SPEC = module_spec(tasks=[
  task("local_task", steps=[cmd("echo", "ok")], execution=LocalOnly(LOCAL)),
  task("remote_task", steps=[cmd("echo", "ok")], execution=RemoteOnly(REMOTE)),
])
SPEC
"#,
    )
    .expect("write tasks");

    let spec = load_workspace(temp.path(), &LoadOptions::default()).expect("load");
    let local_label = parse_label("//apps/web:local_task", "//").expect("local label");
    let remote_label = parse_label("//apps/web:remote_task", "//").expect("remote label");
    assert!(
        spec.tasks.contains_key(&local_label),
        "expected canonical local task to load"
    );
    assert!(
        spec.tasks.contains_key(&remote_label),
        "expected canonical remote task to load"
    );
}

/// Ensures unsupported execution shape mixes are rejected at load time.
#[test]
fn rejects_cross_constructor_execution_shapes() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("apps/web")).expect("mkdir");
    fs::write(
        temp.path().join("apps/web/TASKS.py"),
        r#"
LOCAL = Local(id="dev-local", max_parallel_tasks=2)
REMOTE = Remote(id="remote-a", endpoint="http://127.0.0.1:8001")

SPEC = module_spec(tasks=[
  task("bad_local_shape", steps=[cmd("echo", "ok")], execution=LocalOnly(REMOTE)),
  task("bad_remote_shape", steps=[cmd("echo", "ok")], execution=RemoteOnly(LOCAL)),
])
SPEC
"#,
    )
    .expect("write tasks");

    let err = load_workspace(temp.path(), &LoadOptions::default())
        .expect_err("cross-constructor execution shapes must fail at load time");
    let message = err.to_string();
    assert!(
        message.contains("LocalOnly")
            || message.contains("RemoteOnly")
            || message.contains("unknown field"),
        "unexpected error message: {message}"
    );
}

/// Ensures `ByCustomPolicy` accepts only V1 decision helper outputs.
#[test]
fn policy_helpers_compile_to_v1_decision_ir_variants() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("apps/web")).expect("mkdir");
    fs::write(
        temp.path().join("apps/web/TASKS.py"),
        r#"
REMOTE_A = Remote(id="remote-a", endpoint="http://127.0.0.1:8001")
REMOTE_B = Remote(id="remote-b", endpoint="http://127.0.0.1:8002")
POLICY_CONTEXT = PolicyContext(
  remotes={
    "remote-a": RemoteRuntimeView(endpoint="http://127.0.0.1:8001", healthy=True, queue_eta_s=1.0),
    "remote-b": RemoteRuntimeView(endpoint="http://127.0.0.1:8002", healthy=True, queue_eta_s=2.0),
  },
  remote_any_reachable=True,
)

def choose_local(ctx):
    return Decision_local(reason="LOCAL_FOR_TEST")

def choose_remote(ctx):
    return Decision_remote("remote-a", reason="REMOTE_FOR_TEST")

def choose_remote_any(ctx):
    return Decision_remote_any(["remote-a", "remote-b"], reason="REMOTE_ANY_FOR_TEST")

SPEC = module_spec(tasks=[
  task("policy_local", steps=[cmd("echo", "ok")], execution=ByCustomPolicy(choose_local)),
  task("policy_remote", steps=[cmd("echo", "ok")], execution=ByCustomPolicy(choose_remote)),
  task("policy_remote_any", steps=[cmd("echo", "ok")], execution=ByCustomPolicy(choose_remote_any)),
])
SPEC
"#,
    )
    .expect("write tasks");

    let spec = load_workspace(temp.path(), &LoadOptions::default()).expect("load");

    let local_label = parse_label("//apps/web:policy_local", "//").expect("local label");
    let local = spec.tasks.get(&local_label).expect("local task");
    match &local.execution {
        TaskExecutionSpec::ByCustomPolicy {
            decision: Some(PolicyDecisionSpec::Local { reason }),
            ..
        } => assert_eq!(reason, "LOCAL_FOR_TEST"),
        other => panic!("expected local policy decision, got: {other:?}"),
    }

    let remote_label = parse_label("//apps/web:policy_remote", "//").expect("remote label");
    let remote = spec.tasks.get(&remote_label).expect("remote task");
    match &remote.execution {
        TaskExecutionSpec::ByCustomPolicy {
            decision:
                Some(PolicyDecisionSpec::Remote {
                    reason,
                    remote: selected,
                }),
            ..
        } => {
            assert_eq!(reason, "REMOTE_FOR_TEST");
            assert_eq!(selected.id, "remote-a");
            assert_eq!(selected.endpoint.as_deref(), Some("http://127.0.0.1:8001"));
        }
        other => panic!("expected remote policy decision, got: {other:?}"),
    }

    let remote_any_label = parse_label("//apps/web:policy_remote_any", "//").expect("any label");
    let remote_any = spec.tasks.get(&remote_any_label).expect("any task");
    match &remote_any.execution {
        TaskExecutionSpec::ByCustomPolicy {
            decision: Some(PolicyDecisionSpec::RemoteAny { reason, remotes }),
            ..
        } => {
            assert_eq!(reason, "REMOTE_ANY_FOR_TEST");
            assert_eq!(remotes.len(), 2);
            assert_eq!(remotes[0].id, "remote-a");
            assert_eq!(remotes[1].id, "remote-b");
        }
        other => panic!("expected remote_any policy decision, got: {other:?}"),
    }
}

/// Ensures builder-style policy APIs are rejected with explicit diagnostics.
#[test]
fn rejects_builder_style_policy_api_calls() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("apps/web")).expect("mkdir");
    fs::write(
        temp.path().join("apps/web/TASKS.py"),
        r#"
def choose_runtime(ctx):
    return Decision_start()

SPEC = module_spec(tasks=[
  task("bad_builder_policy", steps=[cmd("echo", "ok")], execution=ByCustomPolicy(choose_runtime)),
])
SPEC
"#,
    )
    .expect("write tasks");

    let err = load_workspace(temp.path(), &LoadOptions::default())
        .expect_err("builder-style policy APIs must fail at load time");
    let message = err.to_string();
    assert!(
        message.contains("unsupported policy builder API: Decision.start"),
        "unexpected error message: {message}"
    );
}

/// Ensures policy decisions with scoring fields are rejected instead of ignored.
#[test]
fn rejects_policy_decisions_with_scoring_fields() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("apps/web")).expect("mkdir");
    fs::write(
        temp.path().join("apps/web/TASKS.py"),
        r#"
def choose_runtime(ctx):
    return {
        "mode": "local",
        "reason": "LOCAL_WITH_SCORE",
        "score": 10,
        "weight": 0.5,
    }

SPEC = module_spec(tasks=[
  task("bad_scoring_policy", steps=[cmd("echo", "ok")], execution=ByCustomPolicy(choose_runtime)),
])
SPEC
"#,
    )
    .expect("write tasks");

    let err = load_workspace(temp.path(), &LoadOptions::default())
        .expect_err("policy scoring fields must be rejected");
    let message = err.to_string();
    assert!(
        message.contains("unsupported policy scoring fields"),
        "unexpected error message: {message}"
    );
    assert!(
        message.contains("score"),
        "diagnostic should mention score field: {message}"
    );
    assert!(
        message.contains("weight"),
        "diagnostic should mention weight field: {message}"
    );
}

/// Ensures V1 rejects unsupported remote workspace transfer modes at load time.
#[test]
fn rejects_unsupported_remote_workspace_transfer_mode() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("apps/web")).expect("mkdir");
    fs::write(
        temp.path().join("apps/web/TASKS.py"),
        r#"
REMOTE = Remote(
  id="remote-a",
  endpoint="http://127.0.0.1:8001",
  workspace={"transfer": "FULL_COPY"},
)

SPEC = module_spec(tasks=[
  task("bad_transfer_mode", steps=[cmd("echo", "ok")], execution=RemoteOnly(REMOTE)),
])
SPEC
"#,
    )
    .expect("write tasks");

    let err = load_workspace(temp.path(), &LoadOptions::default())
        .expect_err("unsupported workspace transfer mode must fail");
    let message = err.to_string();
    assert!(
        message.contains("Remote.workspace.transfer"),
        "diagnostic should identify workspace transfer field: {message}"
    );
    assert!(
        message.contains("REPO_ZIP_SNAPSHOT"),
        "diagnostic should mention supported transfer mode: {message}"
    );
}

/// Ensures V1 rejects unsupported remote result sync modes at load time.
#[test]
fn rejects_unsupported_remote_result_sync_mode() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("apps/web")).expect("mkdir");
    fs::write(
        temp.path().join("apps/web/TASKS.py"),
        r#"
REMOTE = Remote(
  id="remote-a",
  endpoint="http://127.0.0.1:8001",
  result={"sync": "OUTPUTS_ONLY"},
)

SPEC = module_spec(tasks=[
  task("bad_result_mode", steps=[cmd("echo", "ok")], execution=RemoteOnly(REMOTE)),
])
SPEC
"#,
    )
    .expect("write tasks");

    let err = load_workspace(temp.path(), &LoadOptions::default())
        .expect_err("unsupported result sync mode must fail");
    let message = err.to_string();
    assert!(
        message.contains("Remote.result.sync"),
        "diagnostic should identify result sync field: {message}"
    );
    assert!(
        message.contains("OUTPUTS_AND_LOGS"),
        "diagnostic should mention supported sync mode: {message}"
    );
}

/// Ensures `RemoteOnly([])` fails with a clear validation error.
#[test]
fn rejects_remote_only_empty_list() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("apps/web")).expect("mkdir");
    fs::write(
        temp.path().join("apps/web/TASKS.py"),
        r#"
SPEC = module_spec(tasks=[
  task("bad_empty_remote_list", steps=[cmd("echo", "ok")], execution=RemoteOnly([])),
])
SPEC
"#,
    )
    .expect("write tasks");

    let err = load_workspace(temp.path(), &LoadOptions::default())
        .expect_err("empty remote list must fail validation");
    assert!(
        err.to_string()
            .contains("execution RemoteOnly.remote list cannot be empty"),
        "unexpected error message: {err}"
    );
}

/// Ensures container runtime image references are normalized when loading V1 runtime specs.
#[test]
fn normalizes_container_runtime_image_digest_reference() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("apps/web")).expect("mkdir");
    fs::write(
        temp.path().join("apps/web/TASKS.py"),
        r#"
REMOTE = Remote(
  id="remote-a",
  endpoint="http://127.0.0.1:8001",
  runtime=ContainerRuntime(
    image=" GHCR.IO/acme/api@SHA256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA "
  ),
)

SPEC = module_spec(tasks=[
  task("remote_container", steps=[cmd("echo", "ok")], execution=RemoteOnly(REMOTE)),
])
SPEC
"#,
    )
    .expect("write tasks");

    let spec = load_workspace(temp.path(), &LoadOptions::default()).expect("load");
    let label = parse_label("//apps/web:remote_container", "//").expect("label");
    let task = spec.tasks.get(&label).expect("task exists");
    let runtime = match &task.execution {
        TaskExecutionSpec::RemoteOnly(RemoteSelectionSpec::Single(remote)) => {
            remote.runtime.as_ref().expect("runtime exists")
        }
        other => panic!("expected remote runtime task, got: {other:?}"),
    };

    match runtime {
        RemoteRuntimeSpec::Containerized { image } => assert_eq!(
            image,
            "ghcr.io/acme/api@sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        ),
    }
}

/// Ensures malformed runtime image digest references fail at load time.
#[test]
fn rejects_container_runtime_image_with_malformed_digest() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("apps/web")).expect("mkdir");
    fs::write(
        temp.path().join("apps/web/TASKS.py"),
        r#"
REMOTE = Remote(
  id="remote-a",
  endpoint="http://127.0.0.1:8001",
  runtime=ContainerRuntime(image="ghcr.io/acme/api@sha256:abc"),
)

SPEC = module_spec(tasks=[
  task("bad_runtime_image", steps=[cmd("echo", "ok")], execution=RemoteOnly(REMOTE)),
])
SPEC
"#,
    )
    .expect("write tasks");

    let err = load_workspace(temp.path(), &LoadOptions::default())
        .expect_err("malformed digest should fail validation");
    assert!(
        err.to_string().contains("Remote.runtime.image"),
        "diagnostic should identify runtime image field: {err}"
    );
}

/// Ensures invalid container runtime mount/resource/env settings fail during loader validation.
#[test]
fn rejects_invalid_container_runtime_execution_spec_fields() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("apps/web")).expect("mkdir");
    fs::write(
        temp.path().join("apps/web/TASKS.py"),
        r#"
REMOTE = Remote(
  id="remote-a",
  endpoint="http://127.0.0.1:8001",
  runtime=ContainerRuntime(
    image="tak/test:v1",
    mounts=[{"source": "./workspace", "target": "work/src"}],
    resources={"cpu_cores": 0, "memory_mb": 0},
  ),
)

SPEC = module_spec(tasks=[
  task("bad_runtime_fields", steps=[cmd("echo", "ok")], execution=RemoteOnly(REMOTE)),
])
SPEC
"#,
    )
    .expect("write tasks");

    let err = load_workspace(temp.path(), &LoadOptions::default())
        .expect_err("invalid runtime spec fields must fail at load time");
    let message = err.to_string();
    assert!(
        message.contains("Remote.runtime"),
        "diagnostic should identify runtime field group: {message}"
    );
    assert!(
        message.contains("mount") || message.contains("cpu") || message.contains("memory"),
        "diagnostic should identify invalid runtime sub-field: {message}"
    );
}

/// Ensures token-like env values are redacted in runtime validation diagnostics.
#[test]
fn redacts_sensitive_container_runtime_env_values_in_loader_errors() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("apps/web")).expect("mkdir");
    fs::write(
        temp.path().join("apps/web/TASKS.py"),
        r#"
REMOTE = Remote(
  id="remote-a",
  endpoint="http://127.0.0.1:8001",
  runtime=ContainerRuntime(
    image="tak/test:v1",
    env={"SERVICE_TOKEN": "super-secret-token\u0000"},
  ),
)

SPEC = module_spec(tasks=[
  task("bad_runtime_secret", steps=[cmd("echo", "ok")], execution=RemoteOnly(REMOTE)),
])
SPEC
"#,
    )
    .expect("write tasks");

    let err = load_workspace(temp.path(), &LoadOptions::default())
        .expect_err("invalid env value should fail validation");
    let message = err.to_string();
    assert!(
        message.contains("SERVICE_TOKEN"),
        "diagnostic should identify env key: {message}"
    );
    assert!(
        message.contains("<redacted>"),
        "diagnostic should redact sensitive env values: {message}"
    );
    assert!(
        !message.contains("super-secret-token"),
        "diagnostic must not leak sensitive token values: {message}"
    );
}
