use std::path::{Path, PathBuf};

use tak_make::{
    GoalExecutionFuture, GoalExecutionRequest, GoalExecutor, MakefileReadError, MakefileReader,
    MakefileSource, RunMake, RunMakeError, RunMakeRequest,
};

use crate::fixtures::RecordingMakefileReader;

struct MissingReader;

impl MakefileReader for MissingReader {
    fn read(&self, workspace_root: &Path) -> Result<MakefileSource, MakefileReadError> {
        Err(MakefileReadError::NotFound {
            workspace_root: workspace_root.to_path_buf(),
        })
    }
}

struct NeverExecutor;

impl GoalExecutor for NeverExecutor {
    fn execute(&self, _request: GoalExecutionRequest) -> GoalExecutionFuture<'_> {
        panic!("executor must not run after a reader failure")
    }
}

struct FailingExecutor;

impl GoalExecutor for FailingExecutor {
    fn execute(&self, _request: GoalExecutionRequest) -> GoalExecutionFuture<'_> {
        Box::pin(async { Err(RunMakeError::execution("runtime unavailable")) })
    }
}

fn request() -> RunMakeRequest {
    RunMakeRequest {
        workspace_root: PathBuf::from("/workspace"),
        goal: "test".to_string(),
    }
}

#[tokio::test]
async fn reader_failure_stops_before_goal_execution() {
    let error = RunMake::new(&MissingReader, &NeverExecutor)
        .execute(request())
        .await
        .expect_err("reader failure should propagate")
        .to_string();

    assert!(error.contains("no default Makefile"), "{error}");
}

#[tokio::test]
async fn executor_failure_is_returned_by_the_use_case() {
    let reader = RecordingMakefileReader::new("Makefile", "test:\n");

    let error = RunMake::new(&reader, &FailingExecutor)
        .execute(request())
        .await
        .expect_err("executor failure should propagate")
        .to_string();

    assert!(error.contains("runtime unavailable"), "{error}");
}
