use std::path::PathBuf;

use tak_make::{GoalAnnotations, RunMake, RunMakeRequest};

use crate::fixtures::{EXECUTOR_EXIT_CODE, RecordingGoalExecutor, RecordingMakefileReader};

#[tokio::test]
async fn literal_goal_is_executed_with_make_and_default_annotations() {
    let workspace_root = PathBuf::from("/workspaces/example");
    let makefile_path = PathBuf::from("config/Project.mk");
    let reader = RecordingMakefileReader::new(&makefile_path, "test: build\n");
    let executor = RecordingGoalExecutor::new();

    let outcome = RunMake::new(&reader, &executor)
        .execute(RunMakeRequest {
            workspace_root: workspace_root.clone(),
            goal: "test".to_string(),
        })
        .await
        .expect("run make goal");

    assert_eq!(reader.roots(), vec![workspace_root.clone()]);
    let execution = executor.take_request();
    assert_eq!(execution.workspace_root, workspace_root);
    assert_eq!(execution.makefile_path, makefile_path);
    assert_eq!(execution.argv, ["make", "test"]);
    assert_eq!(execution.annotations, GoalAnnotations::default());
    assert_eq!(outcome.exit_code, EXECUTOR_EXIT_CODE);
}
