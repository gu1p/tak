use tak_make::ParallelOutputMode;

use crate::parallel_fixtures::plan_source;

#[tokio::test]
async fn parallel_annotation_builds_children_and_a_make_join() {
    let plan = plan_source(
        ".PHONY: all lint test\n\
         lint:\n\
         test:\n\
         # tak: parallel=lint,test\n\
         all: lint test generated.txt\n",
        "all",
    )
    .await;

    assert_eq!(plan.root_goal, "all");
    assert_eq!(plan.goals.len(), 3);
    let goal = |name| {
        plan.goals
            .iter()
            .find(|candidate| candidate.goal == name)
            .expect("planned goal")
    };
    let lint = goal("lint");
    let test = goal("test");
    let all = goal("all");
    assert_eq!(lint.argv, ["make", "lint"]);
    assert_eq!(test.argv, ["make", "test"]);
    assert_eq!(all.dependencies, ["lint", "test"]);
    assert_eq!(
        all.argv,
        ["make", "--assume-old=lint", "--assume-old=test", "all"]
    );
    assert_eq!(all.parallel_output, ParallelOutputMode::Live);
}
