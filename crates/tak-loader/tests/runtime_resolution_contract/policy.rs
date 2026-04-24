use std::fs;

use tak_core::model::{ContainerRuntimeSourceSpec, PolicyDecisionSpec, RemoteRuntimeSpec};
use tak_loader::evaluate_named_policy_decision;

#[test]
fn evaluates_named_policy_to_local_runtime_selector() {
    let temp = tempfile::tempdir().expect("tempdir");
    let tasks_file = temp.path().join("TASKS.py");
    fs::write(
        &tasks_file,
        r#"def choose_local(ctx):
  return Decision.local(
    reason=Reason.DEFAULT_LOCAL_POLICY,
    runtime=Runtime.Dockerfile(path("docker/Dockerfile")),
  )
"#,
    )
    .expect("write tasks");

    match evaluate_named_policy_decision(&tasks_file, "//", "choose_local")
        .expect("evaluate policy")
    {
        PolicyDecisionSpec::Local { reason, local } => {
            assert_eq!(reason, "DEFAULT_LOCAL_POLICY");
            let runtime = local
                .expect("local selector")
                .runtime
                .expect("local runtime");
            match runtime {
                RemoteRuntimeSpec::Containerized { source } => match source {
                    ContainerRuntimeSourceSpec::Dockerfile {
                        dockerfile,
                        build_context,
                    } => {
                        assert_eq!(dockerfile.path, "docker/Dockerfile");
                        assert_eq!(build_context.path, ".");
                    }
                    other => panic!("expected dockerfile source, got {other:?}"),
                },
            }
        }
        other => panic!("expected local policy, got {other:?}"),
    }
}
