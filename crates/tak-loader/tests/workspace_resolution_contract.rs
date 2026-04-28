use std::fs;

use tak_core::model::{IgnoreSourceSpec, PathAnchor, PolicyDecisionSpec, TaskExecutionSpec};
use tak_loader::{LoadOptions, load_workspace};

#[test]
fn workspace_load_resolves_context_defaults_and_policy_execution() {
    let temp = tempfile::tempdir().expect("tempdir");
    let app_dir = temp.path().join("apps/web");
    fs::create_dir_all(&app_dir).expect("mkdir");
    fs::write(
        temp.path().join("TASKS.py"),
        r#"SPEC = module_spec(
  includes=[path("apps/web")],
  tasks=[],
)
SPEC
"#,
    )
    .expect("write root tasks");
    fs::write(
        app_dir.join("TASKS.py"),
        r#"POLICY_CONTEXT = PolicyContext(local_cpu_percent=92.5)
def choose_remote(ctx):
  return Decision.remote(pool="build", required_tags=["builder"], required_capabilities=["linux"], transport=Transport.DirectHttps(), reason=Reason.LOCAL_CPU_HIGH)
SPEC = module_spec(project_id="proj-alpha", limiters=[resource("cpu", capacity=4, scope=Scope.Project)], queues=[queue_def("deploy", slots=2, scope=Scope.Project)], defaults=Defaults(queue=queue_use("deploy", scope=Scope.Project), retry=retry(attempts=3, backoff=fixed(0)), tags=["default-tag"]), tasks=[task("deploy", steps=[cmd("echo", "ok")], needs=[need("cpu", scope=Scope.Project)], context=CurrentState(roots=[path("//shared"), path("src")], ignored=[gitignore(), path("build")], include=[path("config/settings.json")]), execution=Execution.Decide(choose_remote), tags=["release"])])
SPEC
"#,
    )
    .expect("write tasks");

    let spec = load_workspace(temp.path(), &LoadOptions::default()).expect("load workspace");
    let task = spec.tasks.values().next().expect("resolved task");

    assert_eq!(spec.project_id, "proj-alpha");
    assert_eq!(spec.limiters.len(), 1);
    assert_eq!(spec.queues.len(), 1);
    assert_eq!(task.retry.attempts, 3);
    assert_eq!(task.tags, vec!["default-tag", "release"]);
    assert_eq!(
        task.queue
            .as_ref()
            .expect("queue")
            .queue
            .scope_key
            .as_deref(),
        Some("proj-alpha")
    );
    assert_eq!(task.context.roots[0].anchor, PathAnchor::Workspace);
    assert_eq!(task.context.roots[0].path, "shared");
    assert_eq!(task.context.roots[1].path, "apps/web/src");
    assert!(matches!(
        task.context.ignored[0],
        IgnoreSourceSpec::GitIgnore
    ));
    assert_eq!(
        task.context.include[0].path,
        "apps/web/config/settings.json"
    );
    match &task.execution {
        TaskExecutionSpec::ByCustomPolicy {
            policy_name,
            decision: Some(PolicyDecisionSpec::Remote { reason, remote }),
        } => {
            assert!(!policy_name.is_empty());
            assert_eq!(reason, "LOCAL_CPU_HIGH");
            assert_eq!(remote.pool.as_deref(), Some("build"));
        }
        other => panic!("expected remote policy decision, got {other:?}"),
    }
}
