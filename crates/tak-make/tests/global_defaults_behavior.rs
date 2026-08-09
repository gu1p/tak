use tak_make::{ContainerSource, ExecutionPlacement, GoalAnnotations};

use crate::fixtures::run_source;

#[tokio::test]
async fn goals_inherit_global_defaults_and_may_override_the_container() {
    let source = "# tak: default.execution=remote\n\
                  # tak: default.container-image=alpine:3.20\n\
                  \n\
                  build:\n\
                  \t@:\n\
                  # tak: container-image=debian:bookworm\n\
                  test:\n\
                  \t@:\n";

    let (_, build) = run_source(source, "build").await.expect("run build");
    let (_, test) = run_source(source, "test").await.expect("run test");

    assert_eq!(
        (build.annotations, test.annotations),
        (
            GoalAnnotations {
                placement: Some(ExecutionPlacement::Remote),
                container: Some(ContainerSource::Image {
                    image: "alpine:3.20".to_string(),
                }),
            },
            GoalAnnotations {
                placement: Some(ExecutionPlacement::Remote),
                container: Some(ContainerSource::Image {
                    image: "debian:bookworm".to_string(),
                }),
            },
        )
    );
}

#[tokio::test]
async fn goal_placement_overrides_the_default_and_inherits_its_container() {
    let source = "# tak: default.execution=remote\n\
                  # tak: default.container-image=alpine:3.20\n\
                  # tak: execution=local\n\
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
async fn a_goal_dockerfile_replaces_the_default_image() {
    let source = "# tak: default.execution=remote\n\
                  # tak: default.container-image=alpine:3.20\n\
                  # tak: container-dockerfile=docker/test.Dockerfile\n\
                  # tak: container-build-context=.\n\
                  test:\n\
                  \t@:\n";

    let (_, request) = run_source(source, "test").await.expect("run test");

    assert_eq!(
        request.annotations,
        GoalAnnotations {
            placement: Some(ExecutionPlacement::Remote),
            container: Some(ContainerSource::Dockerfile {
                dockerfile: "docker/test.Dockerfile".to_string(),
                build_context: Some(".".to_string()),
            }),
        }
    );
}
