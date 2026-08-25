use tak_make::{ContainerSource, ExecutionPlacement, ParallelOutputMode};

use crate::parallel_fixtures::plan_source;

#[tokio::test]
async fn nested_groups_inherit_execution_and_nearest_output_mode() {
    let plan = plan_source(
        "# tak: default.parallel-output=live\n\
         .PHONY: all checks build lint test\n\
         lint:\n\
         # tak: execution=local\n\
         test:\n\
         # tak: parallel=lint,test\n\
         # tak: parallel-output=live\n\
         checks: lint test\n\
         build:\n\
         # tak: execution=remote\n\
         # tak: container-image=alpine:3.20\n\
         # tak: parallel-output=grouped\n\
         # tak: parallel=checks,build\n\
         all: checks build\n",
        "all",
    )
    .await;

    let goal = |name| {
        plan.goals
            .iter()
            .find(|candidate| candidate.goal == name)
            .expect("planned goal")
    };
    let lint = goal("lint");
    let test = goal("test");
    let build = goal("build");
    assert_eq!(lint.annotations.placement, Some(ExecutionPlacement::Remote));
    assert_eq!(test.annotations.placement, Some(ExecutionPlacement::Local));
    assert_eq!(build.parallel_output, ParallelOutputMode::Grouped);
    assert_eq!(lint.parallel_output, ParallelOutputMode::Live);
    assert!(matches!(
        lint.annotations.container,
        Some(ContainerSource::Image { .. })
    ));
}
