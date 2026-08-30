mod definitions;
mod environment_values;
mod job;
mod run;
mod submission;
mod validation;
mod workspace;

pub use definitions::*;
pub use environment_values::EnvironmentValue;
pub use job::*;
pub use run::*;
pub use submission::RunSubmission;
pub use workspace::*;

#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct ResolvedRunError {
    message: String,
}

impl ResolvedRunError {
    pub(super) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}
