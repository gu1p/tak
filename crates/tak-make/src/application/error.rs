use std::io;
use std::path::PathBuf;

use crate::domain::MakefileParseError;

/// Failure while loading one of Make's default Makefiles.
#[derive(Debug, thiserror::Error)]
pub enum MakefileReadError {
    /// No default Makefile exists in the requested workspace.
    #[error(
        "no default Makefile found in `{workspace_root}`; searched GNUmakefile, makefile, Makefile"
    )]
    NotFound {
        /// Workspace that was searched.
        workspace_root: PathBuf,
    },
    /// A selected Makefile could not be read as UTF-8 text.
    #[error("failed to read Makefile `{path}`: {source}")]
    Read {
        /// Makefile that failed.
        path: PathBuf,
        /// Underlying filesystem error.
        #[source]
        source: io::Error,
    },
}

/// Failure before or during a Make goal execution.
#[derive(Debug, thiserror::Error)]
pub enum RunMakeError {
    /// The Makefile could not be loaded.
    #[error(transparent)]
    Read(#[from] MakefileReadError),
    /// The requested goal or its Tak annotations were invalid.
    #[error(transparent)]
    Parse(#[from] MakefileParseError),
    /// The injected goal executor failed.
    #[error("Make goal execution failed: {message}")]
    Execution {
        /// Adapter-provided failure detail.
        message: String,
    },
}

impl RunMakeError {
    /// Creates an execution failure reported by a [`crate::GoalExecutor`] adapter.
    ///
    /// ```rust
    /// use tak_make::RunMakeError;
    /// let error = RunMakeError::execution("executor unavailable");
    /// assert!(error.to_string().contains("executor unavailable"));
    /// ```
    pub fn execution(message: impl Into<String>) -> Self {
        Self::Execution {
            message: message.into(),
        }
    }
}
