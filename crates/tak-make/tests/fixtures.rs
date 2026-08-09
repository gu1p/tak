use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tak_make::{
    GoalExecutionFuture, GoalExecutionRequest, GoalExecutor, MakeRunOutcome, MakefileReadError,
    MakefileReader, MakefileSource, RunMake, RunMakeError, RunMakeRequest,
};

pub(crate) const EXECUTOR_EXIT_CODE: i32 = 23;

pub(crate) struct RecordingMakefileReader {
    makefile_path: PathBuf,
    contents: String,
    roots: Mutex<Vec<PathBuf>>,
}

impl RecordingMakefileReader {
    pub(crate) fn new(makefile_path: impl Into<PathBuf>, contents: impl Into<String>) -> Self {
        Self {
            makefile_path: makefile_path.into(),
            contents: contents.into(),
            roots: Mutex::new(Vec::new()),
        }
    }

    pub(crate) fn roots(&self) -> Vec<PathBuf> {
        self.roots.lock().expect("reader roots lock").clone()
    }
}

impl MakefileReader for RecordingMakefileReader {
    fn read(&self, workspace_root: &Path) -> Result<MakefileSource, MakefileReadError> {
        self.roots
            .lock()
            .expect("reader roots lock")
            .push(workspace_root.to_path_buf());
        Ok(MakefileSource {
            makefile_path: self.makefile_path.clone(),
            contents: self.contents.clone(),
        })
    }
}

pub(crate) struct RecordingGoalExecutor {
    request: Mutex<Option<GoalExecutionRequest>>,
}

impl RecordingGoalExecutor {
    pub(crate) fn new() -> Self {
        Self {
            request: Mutex::new(None),
        }
    }

    pub(crate) fn take_request(&self) -> GoalExecutionRequest {
        self.request
            .lock()
            .expect("executor request lock")
            .take()
            .expect("executor request")
    }
}

impl GoalExecutor for RecordingGoalExecutor {
    fn execute(&self, request: GoalExecutionRequest) -> GoalExecutionFuture<'_> {
        self.request
            .lock()
            .expect("executor request lock")
            .replace(request);
        Box::pin(async {
            Ok(MakeRunOutcome {
                exit_code: EXECUTOR_EXIT_CODE,
            })
        })
    }
}

pub(crate) async fn run_source(
    source: &str,
    goal: &str,
) -> Result<(MakeRunOutcome, GoalExecutionRequest), RunMakeError> {
    let reader = RecordingMakefileReader::new("Makefile", source);
    let executor = RecordingGoalExecutor::new();
    let outcome = RunMake::new(&reader, &executor)
        .execute(RunMakeRequest {
            workspace_root: PathBuf::from("/workspace"),
            goal: goal.to_string(),
        })
        .await?;
    Ok((outcome, executor.take_request()))
}
