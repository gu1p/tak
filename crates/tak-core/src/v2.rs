//! Domain types for daemon-owned protocol-v2 runs.

mod authored;
mod environment;
mod placement;
mod resolved_run;
mod runtime;
mod session;

pub use authored::{
    AuthoredDefaults, AuthoredLimiterClaim, AuthoredLimiterDefinition, AuthoredModule,
    AuthoredQueueDefinition, AuthoredQueueUse, AuthoredTask, OutputSelector, Step, TaskContext,
};
pub use environment::PassEnv;
pub use placement::{
    Execution, LocalExecution, RemoteExecution, RemoteRequirements, RemoteSelection,
};
pub use resolved_run::*;
pub use runtime::{ContainerMount, ContainerSource, RuntimeResources, TaskRuntime};
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
    #[error("SessionReuse.Paths requires at least one path")]
    EmptySessionPaths,
    #[error("SessionReuse.Paths selectors must stay inside the workspace")]
    InvalidSessionPath,
    #[error("SharedWorkspace requires matching Affinity.RequireSameNode")]
    SharedWorkspaceRequiresHardAffinity,
    #[error("a SharedWorkspace task cannot weaken or change its session affinity")]
    SharedWorkspaceAffinityOverride,
    #[error("session id must be 1 to 128 bytes with no control characters")]
    InvalidSessionId,
}
