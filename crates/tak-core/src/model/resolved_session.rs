use super::{CurrentStateSpec, OutputSelectorSpec, TaskExecutionSpec};

#[derive(Debug, Clone)]
pub struct SessionUseSpec {
    pub name: String,
    pub display_name: String,
    pub execution: TaskExecutionSpec,
    pub reuse: SessionReuseSpec,
    pub context: Option<CurrentStateSpec>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionLifetimeSpec {
    PerRun,
}

#[derive(Debug, Clone)]
pub enum SessionReuseSpec {
    ShareWorkspace,
    SharePaths { paths: Vec<OutputSelectorSpec> },
}

impl SessionReuseSpec {
    /// Returns a stable user-facing reuse marker.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ShareWorkspace => "share_workspace",
            Self::SharePaths { .. } => "share_paths",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SessionSpec {
    pub name: String,
    pub display_name: String,
    pub execution: TaskExecutionSpec,
    pub reuse: SessionReuseSpec,
    pub lifetime: SessionLifetimeSpec,
    pub context: Option<CurrentStateSpec>,
}
