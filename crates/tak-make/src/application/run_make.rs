use crate::domain::annotations_for_goal;

use super::{
    GoalExecutionRequest, GoalExecutor, MakeRunOutcome, MakefileReader, RunMakeError,
    RunMakeRequest,
};

/// Coordinates Makefile inspection and execution through injected ports.
pub struct RunMake<'a> {
    reader: &'a dyn MakefileReader,
    executor: &'a dyn GoalExecutor,
}

impl<'a> RunMake<'a> {
    /// Creates a Make use case with explicit reader and executor dependencies.
    ///
    /// ```rust
    /// use tak_make::{GoalExecutor, MakefileReader, RunMake};
    ///
    /// fn configured<'a>(
    ///     reader: &'a dyn MakefileReader,
    ///     executor: &'a dyn GoalExecutor,
    /// ) -> RunMake<'a> {
    ///     RunMake::new(reader, executor)
    /// }
    /// ```
    pub fn new(reader: &'a dyn MakefileReader, executor: &'a dyn GoalExecutor) -> Self {
        Self { reader, executor }
    }

    /// Resolves one literal goal and delegates its complete Make invocation.
    ///
    /// ```rust
    /// use std::path::PathBuf;
    /// use tak_make::{MakeRunOutcome, RunMake, RunMakeError, RunMakeRequest};
    ///
    /// async fn run_check(use_case: &RunMake<'_>) -> Result<MakeRunOutcome, RunMakeError> {
    ///     use_case.execute(RunMakeRequest {
    ///         workspace_root: PathBuf::from("."),
    ///         goal: "check".to_string(),
    ///     }).await
    /// }
    /// ```
    pub async fn execute(&self, request: RunMakeRequest) -> Result<MakeRunOutcome, RunMakeError> {
        let source = self.reader.read(&request.workspace_root)?;
        let annotations = annotations_for_goal(&source.contents, &request.goal)?;
        self.executor
            .execute(GoalExecutionRequest {
                workspace_root: request.workspace_root,
                makefile_path: source.makefile_path,
                argv: vec!["make".to_string(), request.goal],
                annotations,
            })
            .await
    }
}
