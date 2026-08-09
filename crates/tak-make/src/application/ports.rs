use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use crate::domain::GoalAnnotations;

use super::{MakefileReadError, RunMakeError};

/// Text and identity of the Makefile selected for one run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MakefileSource {
    /// Workspace-relative Makefile path.
    pub makefile_path: PathBuf,
    /// UTF-8 Makefile contents.
    pub contents: String,
}

/// Reads the Makefile that Tak will inspect.
pub trait MakefileReader: Send + Sync {
    /// Loads a Makefile relative to `workspace_root`.
    ///
    /// ```no_run
    /// # // Reason: this example reads the caller's current directory.
    /// # fn main() -> Result<(), tak_make::MakefileReadError> {
    /// use std::path::Path;
    /// use tak_make::{FilesystemMakefileReader, MakefileReader};
    ///
    /// let reader = FilesystemMakefileReader;
    /// let source = reader.read(Path::new("."))?;
    /// println!("selected {}", source.makefile_path.display());
    /// # Ok(())
    /// # }
    /// ```
    fn read(&self, workspace_root: &Path) -> Result<MakefileSource, MakefileReadError>;
}

/// Inputs to the Make orchestration use case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunMakeRequest {
    /// Workspace containing the Makefile.
    pub workspace_root: PathBuf,
    /// Literal Make goal requested by the user.
    pub goal: String,
}

/// Fully resolved invocation handed to an execution adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalExecutionRequest {
    /// Workspace in which Make executes.
    pub workspace_root: PathBuf,
    /// Makefile path selected by the reader.
    pub makefile_path: PathBuf,
    /// Executable and arguments for the invocation.
    pub argv: Vec<String>,
    /// Tak execution metadata resolved for the requested goal.
    pub annotations: GoalAnnotations,
}

/// Observable completion of a Make invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MakeRunOutcome {
    /// Process exit code, including nonzero Make failures.
    pub exit_code: i32,
}

/// Future returned by a [`GoalExecutor`].
pub type GoalExecutionFuture<'a> =
    Pin<Box<dyn Future<Output = Result<MakeRunOutcome, RunMakeError>> + Send + 'a>>;

/// Executes one fully resolved Make invocation.
pub trait GoalExecutor: Send + Sync {
    /// Starts execution and returns its eventual outcome.
    ///
    /// ```rust
    /// use std::path::PathBuf;
    /// use tak_make::{
    ///     GoalAnnotations, GoalExecutionFuture, GoalExecutionRequest, GoalExecutor,
    ///     MakeRunOutcome,
    /// };
    ///
    /// struct Succeed;
    /// impl GoalExecutor for Succeed {
    ///     fn execute(&self, _request: GoalExecutionRequest) -> GoalExecutionFuture<'_> {
    ///         Box::pin(async { Ok(MakeRunOutcome { exit_code: 0 }) })
    ///     }
    /// }
    ///
    /// let pending = Succeed.execute(GoalExecutionRequest {
    ///     workspace_root: PathBuf::from("."),
    ///     makefile_path: PathBuf::from("Makefile"),
    ///     argv: vec!["make".into(), "check".into()],
    ///     annotations: GoalAnnotations::default(),
    /// });
    /// drop(pending);
    /// ```
    fn execute(&self, request: GoalExecutionRequest) -> GoalExecutionFuture<'_>;
}
