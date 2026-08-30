//! Domain types for daemon-owned protocol-v2 runs.

mod authored;
mod environment;
mod placement;
mod resolved_run;
mod session;

pub use authored::{AuthoredDefaults, AuthoredModule, AuthoredTask, OutputSelector, Step};
pub use environment::PassEnv;
pub use placement::{Execution, LocalExecution, RemoteExecution, RemoteSelection};
pub use resolved_run::*;
pub use session::{Affinity, Session, SessionReuse};

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ValidationError {
    #[error("invalid environment variable name `{0}`")]
    InvalidEnvironmentName(String),
    #[error("affinity group must not be empty")]
    EmptyAffinityGroup,
    #[error("SharedWorkspace max_parallel_tasks must be positive")]
    InvalidSharedParallelism,
    #[error("SharedWorkspace requires matching Affinity.RequireSameNode")]
    SharedWorkspaceRequiresHardAffinity,
    #[error("a SharedWorkspace task cannot weaken or change its session affinity")]
    SharedWorkspaceAffinityOverride,
    #[error("session id must be 1 to 128 bytes with no control characters")]
    InvalidSessionId,
}
