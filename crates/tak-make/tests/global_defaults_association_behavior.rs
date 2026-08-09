use tak_make::{ContainerSource, ExecutionPlacement, GoalAnnotations};

use crate::fixtures::run_source;

#[tokio::test]
async fn a_global_default_does_not_discard_an_adjacent_goal_override() {
    let source = "# tak: execution=local\n\
                  # tak: default.container-image=alpine:3.20\n\
                  test:\n\
                  \t@:\n";

    let (_, request) = run_source(source, "test").await.expect("run test");

    assert_eq!(
        request.annotations,
        GoalAnnotations {
            placement: Some(ExecutionPlacement::Local),
            container: Some(ContainerSource::Image {
                image: "alpine:3.20".to_string(),
            }),
        }
    );
}

#[tokio::test]
async fn a_goal_build_context_overrides_the_default_dockerfile_context() {
    let source = "# tak: default.execution=remote\n\
                  # tak: default.container-dockerfile=docker/Dockerfile\n\
                  # tak: default.container-build-context=.\n\
                  # tak: container-build-context=docker/test\n\
                  test:\n\
                  \t@:\n";

    let (_, request) = run_source(source, "test").await.expect("run test");

    assert_eq!(
        request.annotations,
        GoalAnnotations {
            placement: Some(ExecutionPlacement::Remote),
            container: Some(ContainerSource::Dockerfile {
                dockerfile: "docker/Dockerfile".to_string(),
                build_context: Some("docker/test".to_string()),
            }),
        }
    );
}
