use std::path::PathBuf;

use tak_core::v2::{
    ContainerSource, OutputSelector, PlacementKind, SessionReuse, Step, TaskRuntime,
};
use tak_make::{ContainerSource as MakeContainer, GoalAnnotations, GoalExecutionRequest};

use super::{resolved, task};

#[tokio::test]
async fn synthetic_make_task_becomes_one_concrete_local_v2_job() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    std::fs::write(temp.path().join("Makefile"), "check:\n\t@true\n").unwrap();
    let request = GoalExecutionRequest {
        workspace_root: temp.path().to_path_buf(),
        makefile_path: PathBuf::from("Makefile"),
        argv: vec!["make".into(), "check".into()],
        annotations: GoalAnnotations {
            placement: None,
            container: Some(MakeContainer::Image {
                image: "alpine:3.20".into(),
            }),
        },
    };
    let (spec, target) = task::make_workspace(request).unwrap();
    let bundle = crate::cli::daemon_run::build_workspace(temp.path()).unwrap();

    let submission = resolved::submission(&spec, &[target], 1, false, &[], bundle.descriptor)
        .await
        .unwrap();

    assert_eq!(submission.run.targets, ["//:make"]);
    let Step::Cmd { argv, .. } = &submission.run.tasks[0].steps[0] else {
        panic!("synthetic Make task must be a command")
    };
    assert_eq!(argv, &["make", "check"]);
    assert_eq!(
        submission.run.jobs[0].placement_candidates[0].kind,
        PlacementKind::Local
    );
    assert_eq!(
        submission.run.tasks[0].runtime,
        Some(TaskRuntime::container(ContainerSource::Image {
            image: "alpine:3.20".into(),
        }))
    );
    assert_eq!(
        submission.run.tasks[0].outputs,
        [OutputSelector::Glob { value: "**".into() }]
    );
    assert!(matches!(
        submission.run.jobs[0].session.as_ref().unwrap().reuse,
        SessionReuse::SharedWorkspace { max_parallel_tasks } if max_parallel_tasks.get() == 1
    ));
}
